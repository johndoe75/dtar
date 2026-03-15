use crate::error::DtarError;
use crate::map::{Archive, DedupMap, FileInfo, FileMap};
use crate::{hasher, walker};
use crate::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use size::Size;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use tar::{Builder, EntryType, Header};

/// Creates a deduplicated archive file using the specified directories.
///
/// # Arguments
///
/// - `archive` (`String`): The name of the resulting archive file.
/// - `directories` (`Vec<String>`): A list of directories to collect files from for archiving.
///   Only the first directory in the list is currently used.
/// - `create_dedup_map` (`bool`): If `true`, generates a deduplication map alongside the archive.
/// - `_verbose` (`bool`): Currently unused. Reserved for future verbose output capability.
///
/// # Returns
///
/// - `Result<()>`: Returns `Ok(())` if the archive is created successfully,
///   or an error if the process fails at any stage.
///
/// # Errors
///
/// - Returns an error if no directories are specified in the `directories` parameter.
/// - Returns an error if files cannot be collected from the specified directory.
/// - Returns an error if the hash map generation or tar-writing process fails.
/// - Returns an error if saving the deduplication map fails (when `create_dedup_map` is `true`).
///
/// # Behavior
///
/// This function performs the following steps:
/// 1. Collects all files from the first directory in the `directories` list.
/// 2. Generates a hash map for deduplication using parallel processing:
///    - Each file's hash is calculated.
///    - Files with the same hash are grouped together.
/// 3. Creates a tar archive:
///    - For deduplicated files:
///      - Writes the first instance of the file data.
///      - Writes hard links for duplicates.
///    - For non-duplicated files:
///      - Writes the file as-is.
/// 4. Optionally generates a deduplication map and saves it if `create_dedup_map` is `true`.
/// 5. Outputs the original size, deduplicated size, and percentage of space saved.
///
/// # Notes
///
/// - The tar-writing process is performed sequentially and cannot be parallelized.
/// - If `create_dedup_map` is `true` but the archive is written to stdout, the deduplication map
///   is not created and a warning will be displayed.
/// - Space savings are printed at the end, showing the original size, deduplicated size,
///   and percentage saved.
///
/// # Logging
///
/// - Outputs progress and warnings to `stderr` for user visibility during execution.
pub fn create_archive(
    archive: String,
    directories: Vec<String>,
    create_dedup_map: bool,
    verbose: bool,
) -> Result<()> {
    let mut all_files: Vec<FileInfo> = vec![];

    for dir in directories {
        eprintln!("Collecting files from {}", dir);
        all_files.extend(walker::collect_files(&dir)?);
    }

    // Parallelize the hash map generation:
    // - map: calculate the hash for each file
    // - fold: each thread generates its own local hash map for a chunk of files
    // - reduce: combines all thread-local hash maps into one final map
    eprintln!("Generating hashes for deduplication");
    let map = all_files
        .par_iter()
        .map(hasher::calc_file_hash)
        .flatten()
        .fold(HashMap::new, |acc: FileMap, file: FileInfo| {
            hasher::insert_into_file_map(acc, file)
        })
        .reduce(HashMap::new, |mut map1, map2| {
            for (hash, mut files) in map2 {
                map1.entry(hash).or_default().append(&mut files);
            }
            map1
        });

    eprintln!("Writing the archive to {}", archive);

    let writer = create_tar_writer(&archive)?;
    let mut builder = Builder::new(writer);
    let mut dedup_map = DedupMap::new();
    let mut archive_data = Archive::new();

    // We cannot parallelize the tar writing! This needs to be done sequentially.
    for (hash, files) in map {
        // Duplicates -- write the first file as a file, create hard links for the rest
        if files.len() > 1 {
            let primary = &files[0];
            let sanitized_path = primary.sanitize_path();
            if verbose {
                eprintln!("a {}", sanitized_path);
            }
            builder.append_path_with_name(&primary.path_as_string(), &sanitized_path)?;
            archive_data.add_file(&primary);

            if create_dedup_map {
                dedup_map.add_file(&hash, PathBuf::from(&files[0].path_as_string()));
            }

            for dup in &files[1..] {
                let dup_sanitized_path = dup.sanitize_path();

                if verbose {
                    eprintln!("h {}", dup_sanitized_path);
                }

                let mut header = Header::new_gnu();
                header.set_uid(<u64>::from(dup.dir_entry.metadata()?.uid()));
                header.set_gid(<u64>::from(dup.dir_entry.metadata()?.gid()));
                header.set_entry_type(EntryType::Link);
                header.set_size(0);

                builder.append_link(
                    &mut header,
                    &dup_sanitized_path,
                    &primary.path_as_string(),
                )?;
                archive_data.add_dup(&dup);

                if create_dedup_map {
                    dedup_map.add_file(&hash, PathBuf::from(&dup.path_as_string()));
                }
            }
        } else {
            let sanitized_path = files[0].sanitize_path();

            if verbose {
                eprintln!("a {}", sanitized_path);
            }

            builder.append_path_with_name(&files[0].path_as_string(), &sanitized_path)?;
            archive_data.add_file(&files[0]);
        }
    }

    builder.finish()?;

    if create_dedup_map {
        if let Some(dedup_map_path) = generate_dedup_map_path(&archive) {
            dedup_map.save(&dedup_map_path)?;
            eprintln!("Deduplication map saved to: {}", dedup_map_path);
        } else {
            eprintln!("Warning: Cannot create deduplication map when writing to stdout");
        }
    }

    eprintln!(
        "\nArchive orig size: {}, dedup size: {}, saved: {:.2} %",
        Size::from_bytes(archive_data.orig_size),
        Size::from_bytes(archive_data.dedup_size),
        ((1.0 - archive_data.get_dedup_ratio()) * 100.0)
    );

    Ok(())
}

/// If the archive path is "-", we write the archive to stdout -- like the GNU tar would.
/// This allows the user to further handle the tar before it hits the drive.  Something like
/// compressing the archive like this:
///
/// dtar - {directory} | gzip > output.tgz
fn create_tar_writer(path: &str) -> Result<Box<dyn Write>> {
    if path == "-" {
        Ok(Box::new(io::stdout()))
    } else {
        Ok(Box::new(File::create(path)?))
    }
}

/// Generates the path for the dedup-map based on the archive name and path.
///
/// Replaces .tar by .ddm (de-dup-map)
/// Adds .ddm if no extension is present in the archive name
/// Returns None if archive is stdout ("-")
fn generate_dedup_map_path(archive: &str) -> Option<String> {
    if archive == "-" {
        return None;
    }

    if archive.ends_with(".tar") {
        Some(archive.replace(".tar", ".ddm"))
    } else {
        Some(format!("{}.ddm", archive))
    }
}
