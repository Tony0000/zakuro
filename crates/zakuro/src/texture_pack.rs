//! putting a texture pack, a .zip or a folder, in a game's textures folder,
//! each in a folder of its own named after it, so packs put in go on
//! together and one put in again takes its old one's place. the textures
//! folder is read whole, the packs in it with it.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use zakuro_gpu::pack::Pack;

/// a pack going in, on a thread of its own, the textures folder read again
/// once it is in.
pub struct Install {
    /// the game it is for.
    pub program_id: u64,
    /// the folder it goes in, which names it.
    pub name: String,
    done: mpsc::Receiver<Result<Option<Pack>, String>>,
}

impl Install {
    /// puts the pack at source in textures, and reads the packs there.
    pub fn start(source: PathBuf, textures: PathBuf, program_id: u64) -> Install {
        let name = pack_name(&source);
        let folder = textures.join(&name);
        let (sent, done) = mpsc::channel();
        std::thread::spawn(move || {
            let read = install(&source, &textures, &folder).map(|()| Pack::open(&textures));
            let _ = sent.send(read);
        });
        Install { program_id, name, done }
    }

    /// the packs in the textures folder once the pack is in, none of them
    /// when there are no pictures, or why it is not in.
    pub fn finished(&self) -> Option<Result<Option<Pack>, String>> {
        match self.done.try_recv() {
            Ok(done) => Some(done),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("the install stopped".into())),
        }
    }
}

/// the folder a pack goes in, its file's or folder's name, kept to what
/// every system takes in a name.
fn pack_name(source: &Path) -> String {
    let stem = source.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    let name: String = stem.chars().map(|c| if c.is_alphanumeric() || " -_.()".contains(c) { c } else { '_' }).collect();
    let name = name.trim().trim_matches('.').to_owned();
    if name.is_empty() { "texture pack".into() } else { name }
}

/// copies or unzips the pack beside its folder first, so a pack that
/// fails leaves the one there, then puts it in that folder's place.
fn install(source: &Path, textures: &Path, folder: &Path) -> Result<(), String> {
    std::fs::create_dir_all(textures).map_err(|error| format!("the textures folder can't be made, {error}"))?;
    let real = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_owned());
    if real(source).starts_with(real(textures)) {
        return Err("it is in the textures folder already".into());
    }
    // beside it, named after it whole, a dot in a pack's name such as a
    // version's is not an extension
    let mut unpacked = folder.as_os_str().to_owned();
    unpacked.push(".installing");
    let unpacked = PathBuf::from(unpacked);
    let _ = std::fs::remove_dir_all(&unpacked);
    let put = if source.is_dir() {
        crate::copy_dir(source, &unpacked).map_err(|error| format!("it can't be copied, {error}"))
    } else {
        unzip(source, &unpacked)
    };
    let checked = put.and_then(|()| match Pack::open(&unpacked) {
        Some(_) => Ok(()),
        None => Err("it has no texture pack pictures, PNGs named the way Citra and Azahar dump textures".into()),
    });
    if let Err(error) = checked {
        let _ = std::fs::remove_dir_all(&unpacked);
        return Err(error);
    }
    if folder.exists() {
        std::fs::remove_dir_all(folder).map_err(|error| format!("the pack of the same name can't make way, {error}"))?;
    }
    std::fs::rename(&unpacked, folder).map_err(|error| format!("it can't be put in place, {error}"))
}

/// a .zip's files, under to, none of them outside it.
fn unzip(zip: &Path, to: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip).map_err(|error| format!("it can't be read, {error}"))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|error| format!("it is not a .zip that can be read, {error}"))?;
    archive.extract(to).map_err(|error| format!("it can't be unzipped, {error}"))?;
    // the Finder's zips carry each file's macOS metadata beside the files
    let _ = std::fs::remove_dir_all(to.join("__MACOSX"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a pack's folder takes its name, with what some systems refuse in
    /// a name swapped out.
    #[test]
    fn packs_are_named_after_their_file() {
        assert_eq!(pack_name(Path::new("/downloads/SMT4 HD Fonts v1.2.zip")), "SMT4 HD Fonts v1.2");
        assert_eq!(pack_name(Path::new("/downloads/a:b*c")), "a_b_c");
        assert_eq!(pack_name(Path::new("/")), "texture pack");
    }

    /// a .zip's pictures go in their own folder, one with none leaves
    /// nothing, and one put in again takes the place of the one there.
    #[test]
    fn a_zipped_pack_goes_in_its_own_folder() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("zakuro packs {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let textures = dir.join("textures");
        let zipped = |name: &str, files: &[&str]| {
            let path = dir.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            for file in files {
                writer.start_file(*file, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
                writer.write_all(b"not really a png").unwrap();
            }
            writer.finish().unwrap();
            path
        };
        let picture = "00040000000E5C00/tex1_128x128_0123456789ABCDEF_12.png";
        let pack = zipped("Fonts 1.2.zip", &[picture, "__MACOSX/00040000000E5C00/._tex1_128x128_0123456789ABCDEF_12.png"]);
        let folder = textures.join("Fonts 1.2");
        assert_eq!(install(&pack, &textures, &folder), Ok(()));
        assert!(folder.join(picture).is_file());
        assert!(!folder.join("__MACOSX").exists(), "the Finder's metadata stays out");
        assert!(!textures.join("Fonts 1.installing").exists() && !textures.join("Fonts 1.2.installing").exists());

        let escaping = zipped("Escaping.zip", &[picture, "../../outside.png"]);
        let _ = install(&escaping, &textures, &textures.join("Escaping"));
        assert!(!dir.join("outside.png").exists() && !dir.parent().unwrap().join("outside.png").exists(), "nothing goes outside the folder");

        let empty = zipped("Empty.zip", &["readme.txt"]);
        assert!(install(&empty, &textures, &textures.join("Empty")).is_err());
        assert!(!textures.join("Empty").exists() && !textures.join("Empty.installing").exists());

        let newer = zipped("Fonts 1.2.zip", &["tex1_64x64_FEDCBA9876543210_12.png"]);
        assert_eq!(install(&newer, &textures, &folder), Ok(()));
        assert!(!folder.join(picture).exists(), "the old pack made way");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
