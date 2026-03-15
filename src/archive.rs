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

pub fn create_archive(
    archive: String,
    directories: Vec<String>,
    create_dedup_map: bool,
    _verbose: bool,
) -> Result<()> {
    let dir = directories
        .first()
        .ok_or_else(|| DtarError::NoDirectories)?;

    eprintln!("Collecting files from {}", dir);
    let all_files = walker::collect_files(dir)?;

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
            eprintln!("a {}", files[0].sanitize_path());
            builder.append_path_with_name(&primary.path_as_string(), &primary.sanitize_path())?;
            archive_data.add_file(&primary);

            if create_dedup_map {
                dedup_map.add_file(&hash, PathBuf::from(&files[0].path_as_string()));
            }

            for dup in &files[1..] {
                eprintln!("h {}", dup.sanitize_path());

                let mut header = Header::new_gnu();
                header.set_uid(<u64>::from(dup.dir_entry.metadata()?.uid()));
                header.set_gid(<u64>::from(dup.dir_entry.metadata()?.gid()));
                header.set_entry_type(EntryType::Link);
                header.set_size(0);

                builder.append_link(
                    &mut header,
                    &dup.sanitize_path(),
                    &primary.path_as_string(),
                )?;
                archive_data.add_dup(&dup);

                if create_dedup_map {
                    dedup_map.add_file(&hash, PathBuf::from(&dup.path_as_string()));
                }
            }
        } else {
            eprintln!("a {}", files[0].sanitize_path());
            builder.append_path_with_name(&files[0].path_as_string(), &files[0].sanitize_path())?;
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
