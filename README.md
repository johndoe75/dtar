# dtar - Deduplicating TAR Archiver

A tar-compatible archiver that automatically deduplicates identical files
using hardlinks.

## Motivation

While excellent backup and archiving solutions like borg and zpaq exist, which
achieve high compression ratios through sophisticated block-level
deduplication, sometimes you just need a simple tar archive that:

- Can be extracted by any standard tar implementation
- Maintains compatibility across systems and time
- Is easy to inspect and verify
- Doesn't require special tools for extraction
- Can be used with tapes

This is where `dtar` comes in. It creates standard tar archives while
automatically detecting and deduplicating identical files using hardlinks.
This provides:

- **Universal Compatibility**: The resulting archives can be extracted with
  GNU tar or any other tar implementation
- **Transparent Storage**: The archive content is not modified or compressed
  in any special way
- **Simple Deduplication**: While not as sophisticated as block-level
  deduplication used by borg/zpaq, file-level deduplication through hardlinks
  still saves considerable space for common use cases
- **Easy Verification**: Standard tools can inspect and verify the archive
  contents

### Why Not Use Existing Solutions?

- **borg**: Excellent for backups with block-level deduplication and
  encryption, but requires borg for extraction
- **zpaq**: Great compression and deduplication, but archives are in a
  specialized format
- **tar**: Universal but no built-in deduplication
- **dtar**: Combines tar's universality with basic deduplication through
  hardlinks

### Reliability Considerations

A major advantage of tar-based archives is their resilience to corruption.
Real-world experiences have shown:

- zpaq archives can be lost entirely due to issues with incremental backups
  and handling errors
- borg repositories can become completely inaccessible if corruption occurs in
  critical metadata sections
- tar archives, even when partially corrupted, often allow:
    - Recovery of undamaged files
    - Partial extraction of the archive
    - Data salvaging using standard tools
    - Recovery of most content even if some files are lost

This resilience is a crucial feature when storing important data, as it
reduces the risk of catastrophic data loss.

### Use Case Examples

- Archiving source code repositories with many identical files across branches
- Backing up document collections with multiple copies
- Creating distributable archives that need to work everywhere
- Situations where archive format longevity is more important than maximum
  compression
- Cases where data recovery options are crucial

## Features

- Creates standard tar archives
- Automatically detects and deduplicates identical files using hardlinks
- Maintains full tar compatibility
- Parallel processing for improved performance
- Stores deduplication information for later reconstruction

## Trade-offs

dtar consciously trades some efficiency for compatibility:

- Only file-level deduplication (vs block-level in specialized tools)
- No compression (use standard tools like gzip if needed)
- Slightly larger metadata overhead

But gains:

- Universal compatibility
- Format longevity
- Tool independence
- Process transparency
- Better resilience against corruption
- Partial recovery options

# 🤝 Fair Use Policy

[![Fair OSS User](https://img.shields.io/badge/Fair--OSS--User-%E2%9C%94-green)](https://yourproject.org/fair-use)

This project is licensed under the Apache 2.0 License and made available as
open source in the spirit
of collaboration and mutual benefit.

We kindly ask all users — especially commercial users — to follow this Fair
Use Policy:

### 💡 If you use this project in production:

- **Please contribute back** improvements, bug fixes, or enhancements whenever
  possible.
- **Please consider sponsoring** the project, supporting long-term maintenance
  and development.
- **Please open issues** if you encounter bugs or have ideas that could help
  others.

### 🔁 If you modify or extend the project:

- **Share your improvements** publicly, if possible.
- If not, **let the maintainers know privately** — we’re open to
  collaboration, even under NDA if needed.

### 🌱 If you benefit from the project:

- Give credit where due (e.g. in documentation or acknowledgments).
- Advocate for open source sustainability in your company or organization.

---

This is not a legal requirement — it's a social contract.

We believe in the open source ecosystem as a shared effort.  
If you benefit from this project, please be fair and help keep it alive.

Thank you for being a responsible open source user.