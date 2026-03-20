use crate::Result;
use crate::hasher::calc_file_hash;
use crate::map::{Archive, DedupMap, FileInfo, FileMap};
use crate::{hasher, walker};
use anyhow::{Context, anyhow, bail};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use sha2::digest::Update;
use sha2::{Digest, Sha256};
use size::Size;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Read, Seek, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tar::{Builder, EntryType, Header};

const ARCHIVE_FILE_EXTENSION: &str = "ddm";
const ARCHIVE_FILE_NAME: &str = "archive";

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
        .into_par_iter()
        .map(FileInfo::with_calculated_hash)
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

    match archive.as_str() {
        "-" => eprintln!("Sending archive to stdout"),
        _ => eprintln!("Writing the archive to {}", archive),
    }

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
            dedup_map.add_file(&hash, PathBuf::from(&sanitized_path));

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

                builder.append_link(&mut header, &dup_sanitized_path, &sanitized_path)?;
                archive_data.add_dup(&dup);
                dedup_map.add_file(&hash, PathBuf::from(&dup_sanitized_path));
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

    // In every case, we add the deduplication map to the archive to keep it as portable as possible.
    let default_name = format!("{}.{}", ARCHIVE_FILE_NAME, ARCHIVE_FILE_EXTENSION);
    let dedup_map_filename =
        generate_dedup_map_path(&archive).unwrap_or_else(|| default_name.clone());
    dedup_map.save(&dedup_map_filename)?;
    builder.append_path_with_name(&dedup_map_filename, default_name)?;
    builder.finish()?;

    // If we shall not create the deduplication map, we remove it from the drive after the archive
    // was created
    if !create_dedup_map {
        std::fs::remove_file(dedup_map_filename)?;
    }

    eprintln!(
        "\nArchive orig size: {}, dedup size: {}, saved: {:.2} %",
        Size::from_bytes(archive_data.orig_size),
        Size::from_bytes(archive_data.dedup_size),
        ((1.0 - archive_data.get_dedup_ratio()) * 100.0)
    );

    Ok(())
}

pub fn extract_archive(extract_to: String, archive: String, _verbose: bool) -> Result<()> {
    let file = File::open(archive)?;
    let mut tar = tar::Archive::new(file);

    eprintln!("Extracting archive to {}", extract_to);

    tar.set_preserve_permissions(true);
    tar.set_preserve_ownerships(true);
    tar.set_preserve_mtime(true);
    tar.unpack(extract_to)?;

    Ok(())
}

pub fn verify_archive(archive: String) -> Result<()> {
    let mut file = File::open(&archive)?;
    let map_name = format!("{}.{}", ARCHIVE_FILE_NAME, ARCHIVE_FILE_EXTENSION);
    let dedup_map = read_dedup_map_from_archive_end(map_name, &mut file)
        .with_context(|| format!("failed to read dedup map from archive '{}'", archive))?;

    file.rewind()?;
    let mut tar = tar::Archive::new(file);

    for entry_result in tar.entries()? {
        let mut entry = entry_result?;
        let header = entry.header();
        let path = entry.path()?.into_owned();

        if path == Path::new(map_name.as_str()) {
            continue;
        }

        match header.entry_type() {
            EntryType::Regular => {
                let file_info = FileInfo::new(entry);

                let mut file = File::open(&entry.path()?)?;
                let hash = calc_file_hash(&mut file)?;

                let hash = match calc_file_hash(&mut entry) {
                    Ok(hash) => hash.to_string(),
                    Err(_) => continue,
                };
                // let hash = sha256_reader(&mut entry)?;
                let expected_original = dedup_map.get_originals(&hash);
                let expected_duplicates = dedup_map.get_duplicates(&hash);

                let path_matches = expected_original == Some(&path)
                    || expected_duplicates
                        .map(|dups| dups.iter().any(|dup| dup == &path))
                        .unwrap_or(false);

                if !path_matches {
                    bail!(
                        "file '{}' has hash '{}' but is not present in the dedup map",
                        path.display(),
                        hash
                    );
                }
            }
            EntryType::Link => {
                let link_name = entry
                    .link_name()?
                    .ok_or_else(|| anyhow!("hard link '{}' has no target", path.display()))?
                    .into_owned();

                let mut found = false;

                for (hash, original) in dedup_map.iter_originals() {
                    if original == &link_name {
                        let duplicates = dedup_map.get_duplicates(hash).ok_or_else(|| {
                            anyhow!("hash '{}' missing duplicates entry in dedup map", hash)
                        })?;

                        if duplicates.iter().any(|dup| dup == &path) {
                            found = true;
                            break;
                        }
                    }
                }

                if !found {
                    bail!(
                        "hard link '{}' -> '{}' is not described by the dedup map",
                        path.display(),
                        link_name.display()
                    );
                }
            }
            _ => {}
        }
    }

    eprintln!("Archive verification successful: all hashes match the dedup map");
    Ok(())
}

fn read_dedup_map_from_archive_end(map_name: String, file: &mut File) -> Result<DedupMap> {
    file.rewind()?;
    let mut tar = tar::Archive::new(file);

    let mut last_path: Option<PathBuf> = None;
    let mut last_conents: Option<Vec<u8>> = None;

    for entry_result in tar.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.into_owned();
        last_path = Some(path);
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        last_conents = Some(contents);
    }

    let last_path = last_path.ok_or_else(|| anyhow!("Archive is empty"))?;
    if last_path != Path::new(&map_name) {
        bail!(
            "dedup map not found at the end of the archive. Last entry was: {}",
            last_path.display()
        );
    }

    let contents = last_conents.ok_or_else(|| anyhow!("archive.ddm was empty"))?;
    let dedup_map: DedupMap =
        serde_json::from_slice(&contents).context("Failed to parse dedup map")?;

    Ok(dedup_map)
}

/// If the archive path is "-", we write the archive to stdout -- like the GNU tar would.
/// This allows the user to further handle the tar before it hits the drive.  Something like
/// compressing the archive like this:
///
/// dtar - {directory} | gzip > output.tgz
fn create_tar_writer(path: &str) -> Result<Box<dyn Write>> {
    match path {
        "-" => Ok(Box::new(io::stdout())),
        _ => Ok(Box::new(File::create(path)?)),
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

    match archive.ends_with(".tar") {
        true => Some(archive.replace(".tar", ".ddm")),
        false => Some(format!("{}.ddm", archive)),
    }
}
