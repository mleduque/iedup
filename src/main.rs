use ansi_term::Colour::{Blue, Green, Red, Yellow};
use anyhow::anyhow;
use anyhow::Error as AnyError;
use anyhow::Result;
use clap::Clap;
use once_cell::sync::Lazy;
use std::ffi::OsStr;
use std::path::Path;
use glob::glob;

#[derive(Clap)]
#[clap(version = "1.0", author = "Mickaël Leduque <mleduque@gmail.com>")]
struct Opts {
    source: String,
    target: String,
}

static MUS_EXT: Lazy<&OsStr> = Lazy::new(|| &OsStr::new("mus"));
static WAV_EXT: Lazy<&OsStr> = Lazy::new(|| &OsStr::new("wav"));
static NO_EXT: Lazy<&OsStr> = Lazy::new(|| &OsStr::new(""));

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let source = Path::new(&opts.source);
    let target = Path::new(&opts.target);

    // ensure source and target are directories
    if !source.is_dir() {
        return Err(anyhow!("source {} is not a directory", opts.source));
    }
    if !target.is_dir() {
        return Err(anyhow!("target {} is not a directory", opts.target));
    }

    // ensure target is empty
    let mut target_files = target.read_dir()?;
    if target_files.next().is_some() {
        return Err(anyhow!("target dir {} is not empty", opts.target));
    }

    // ensure source dir looks like a gog EE infinity engine directory
    let hint = HintStructure {
        os: Os::Linux,
        vendor: Vendor::Gog,
        variant: Variant::Ee,
    };
    check_source(&source, &hint)?;

    // root dir : copy start.sh (allows user modification), link support/ (no changes expected), create game/

    copy_item(source, target, "start.sh")?;
    link_item(source, target, "gameinfo")?;
    link_item(source, target, "support")?;
    process_dlc_zips(source, target)?;
    create_dir_str(target, "game")?;

    process_game_dir(&source.join("game"), &target.join("game"))?;

    Ok(())
}

fn copy_item(source: &Path, target: &Path, item: &str) -> Result<()> {
    copy_item_os(source, target, OsStr::new(item))
}

fn copy_item_os(source: &Path, target: &Path, item: &OsStr) -> Result<()> {
    let source_item = source.join(item);
    let target_item = target.join(item);
    println!(
        "copy {} to {}",
        Blue.bold().paint(source_item.to_string_lossy()),
        Green.paint(target_item.to_string_lossy())
    );
    std::fs::copy(source_item, target_item)?;
    Ok(())
}

fn link_item(source: &Path, target: &Path, item: &str) -> Result<()> {
    link_item_os(source, target, OsStr::new(item))
}

fn link_item_os(source: &Path, target: &Path, item: &OsStr) -> Result<()> {
    let source_item = source.join(item);
    let target_item = target.join(item);
    println!(
        "link {} to {}",
        Blue.bold().paint(source_item.to_string_lossy()),
        Green.paint(target_item.to_string_lossy())
    );
    Ok(std::os::unix::fs::symlink(source.join(item), target.join(item))?)
}

fn create_dir_str(target: &Path, item: &str) -> Result<()> {
    std::fs::create_dir(target.join(item))?;
    Ok(())
}

fn create_dir_os(target: &Path, item: &OsStr) -> Result<()> {
    std::fs::create_dir(target.join(item))?;
    Ok(())
}

fn process_dlc_zips(source: &Path, target: &Path) -> Result<()> {
    link_pattern_files(source, target, "*-dlc.zip")
}

fn link_pattern_files(source: &Path, target: &Path, pattern: &str) -> Result<()> {
    for entry in glob(source.join(pattern).to_str().unwrap())? {
        match entry {
            Ok(path) => {
                if !path.is_dir() {
                    if let Some(name) = path.file_name() {
                        link_item_os(source, target, name)?;
                    }
                }
            }
            Err(err) => {
                println!("{}", Red.bold().paint(format!("{}", err)));
                return Err(err)?;
            }
        }
    }
    Ok(())
}

fn process_game_dir(source: &Path, target: &Path) -> Result<()> {
    println!(
        "{} to {}",
        Blue.bold().paint(source.to_string_lossy()),
        Green.paint(target.to_string_lossy())
    );
    // copy chitin.key and engine.lua which can be modded
    copy_item(source, target, "chitin.key")?;
    copy_item(source, target, "engine.lua")?;
    // the other non-dirs are supposed to be game exe's and will be linked
    let source_files = source.read_dir()?;
    for file in source_files {
        let file = file?;
        if !file.file_type()?.is_dir() && file.file_name() != "chitin.key" && file.file_name() != "engine.lua" {
            link_item_os(source, target, &file.file_name())?;
        }
    }
    // link the dir: Manual
    println!("{}", Blue.bold().paint(" => Manuals/"));
    link_item(source, target, "Manuals")?;
    // create the dir, copy the content: scripts (scripts can be customized, added)
    println!("{}", Blue.bold().paint(" => scripts/"));
    create_dir_str(target, "scripts")?;
    process_scripts_dir(&source.join("scripts"), &target.join("scripts"))?;

    // continue with the other dirs
    // create the dirs: data, lang, movies, music
    println!("{}", Blue.bold().paint(" => data/"));
    create_dir_str(target, "data")?;
    process_data_dir(&source.join("data"), &target.join("data"))?;
    println!("{}", Blue.bold().paint(" => lang/"));
    create_dir_str(target, "lang")?;
    process_lang_dir(&source.join("lang"), &target.join("lang"))?;
    println!("{}", Blue.bold().paint(" => movies/"));
    create_dir_str(target, "movies")?;
    process_movies_dir(&source.join("movies"), &target.join("movies"))?;
    println!("{}", Blue.bold().paint(" => music/"));
    create_dir_str(target, "music")?;
    process_music_dir(&source.join("music"), &target.join("music"))?;
    println!("{}", Blue.bold().paint(" <= done"));

    //create override/ dir anyway
    create_dir_str(target, "override")?;
    //copy content if exists
    let root_override_dir = source.join("override");
    if root_override_dir.exists() {
        println!("{}", Blue.bold().paint(" => override"));
        process_override_dir(&root_override_dir, &target.join("override"))?;
    } else {
        println!("{}", Yellow.paint(format!("no {}", root_override_dir.to_string_lossy())));
    }

    // done
    Ok(())
}

fn process_override_dir(source: &Path, target: &Path) -> Result<()> {
    //copy content
    copy_content(source, target)
}

fn copy_content(source: &Path, target: &Path) -> Result<()> {
    let scripts = source.read_dir()?;
    for file in scripts {
        let file = file?;
        if let Err(error) = copy_item_os(source, target, &file.file_name()) {
            return Err(anyhow!("Error copying file {:?} from {:?} to {:?}\n  ->{:?}", file.file_name(), source, target, error));
        }
    }
    Ok(())
}

fn process_scripts_dir(source: &Path, target: &Path) -> Result<()> {
    //copy content
    copy_content(source, target)
}

fn process_data_dir(source: &Path, target: &Path) -> Result<()> {
    // link all files inside(should all be .bif)
    link_all_inside(source, target)?;
    Ok(())
}

fn process_lang_dir(source: &Path, target: &Path) -> Result<()> {
    // each language in a subdir (for ex. en_US)
    let languages = source.read_dir()?;
    for language in languages {
        let language = language?.file_name();
        create_dir_os(target, &language)?;
        process_language(&source.join(&language), &target.join(&language), &language.to_string_lossy())?;
    }

    Ok(())
}

fn process_language(source: &Path, target: &Path, language_mark: &str) -> Result<()> {
    // in each language subdir,
    // - one dialog.tlk OR dialog.tlk+dialogF.tlk -> copy because those are modifiable
    // - [maybe]one movies subdir with root wbm and lo/ and 480/ -> like movies at root
    // - [maybe]one sounds/ subdir
    // - [maybe]one data/ subdir (ex: de_DE)
    // - [maybe]one override/ subdir (ex: de_DE)

    println!(
        "{} to {}",
        Blue.bold().paint(source.to_string_lossy()),
        Green.paint(target.to_string_lossy())
    );

    copy_non_dirs(source, target)?; // tlk
    let source_movies_dir = source.join("movies");
    if source_movies_dir.exists() {
        let target_movies_dir = target.join("movies");
        println!(
            "{} to {}",
            Blue.bold().paint(source_movies_dir.to_string_lossy()),
            Green.paint(target_movies_dir.to_string_lossy())
        );
        create_dir_str(target, "movies")?;
        process_movies_dir(&source_movies_dir, &target_movies_dir)?;
    } else {
        println!("{}", Yellow.paint(format!("no movies/ for {}", language_mark)));
    }
    let source_sounds_dir = source.join("sounds");
    if source_sounds_dir.exists() {
        let target_sounds_dir = target.join("sounds");
        println!(
            "{} to {}",
            Blue.bold().paint(source_sounds_dir.to_string_lossy()),
            Green.paint(target_sounds_dir.to_string_lossy())
        );
        create_dir_str(target, "sounds")?;
        process_sound_dir(&source_sounds_dir, &target_sounds_dir)?;
    } else {
        println!("{}", Yellow.paint(format!("no sounds/ for {}", language_mark)));
    }
    let source_override_dir = source.join("override");
    if source_override_dir.exists() {
        let target_override_dir = target.join("override");
        println!(
            "{} to {}",
            Blue.bold().paint(source_override_dir.to_string_lossy()),
            Green.paint(target_override_dir.to_string_lossy())
        );
        create_dir_str(target, "override")?;
        process_override_dir(&source_override_dir, &target_override_dir)?;
    } else {
        println!("{}", Yellow.paint(format!("no override/ for {}", language_mark)));
    }
    let source_data_dir = source.join("data");
    if source_data_dir.exists() {
        let target_data_dir = target.join("data");
        println!(
            "{} to {}",
            Blue.bold().paint(source_data_dir.to_string_lossy()),
            Green.paint(target_data_dir.to_string_lossy())
        );
        create_dir_str(target, "data")?;
        process_data_dir(&source_data_dir, &target_data_dir)?;
    } else {
        println!("{}", Yellow.paint(format!("no data/ for {}", language_mark)));
    }
    Ok(())
}

fn process_sound_dir(source: &Path, target: &Path) -> Result<()> {
    // *.wav files and one sndlist.txt -> create dir, link *.wav, copy sndlist.txt
    let files = source.read_dir()?;
    for file in files {
        let file = file?;
        let file_path = file.path();
        let ext = file_path.extension().unwrap_or(&*NO_EXT);
        if ext == *WAV_EXT {
            link_item_os(source, target, &file.file_name())?;
        } else {
            copy_item_os(source, target, &file.file_name())?;
        }
    }

    Ok(())
}
fn link_non_dirs(source: &Path, target: &Path) -> Result<()> {
    let files = source.read_dir()?;
    for file in files {
        let file = file?;
        if !file.file_type()?.is_dir() {
            link_item_os(source, target, &file.file_name())?;
        }
    }
    Ok(())
}
fn copy_non_dirs(source: &Path, target: &Path) -> Result<()> {
    let files = source.read_dir()?;
    for file in files {
        let file = file?;
        if !file.file_type()?.is_dir() {
            copy_item_os(source, target, &file.file_name())?;
        }
    }
    Ok(())
}

fn process_movies_dir(source: &Path, target: &Path) -> Result<()> {
    // on set of movies at the root, one in 480, one in lo
    // link all root movies (non-dir files)
    link_non_dirs(source, target)?;
    let source_480 = source.join("480");
    if source_480.exists() {
        create_dir_str(target, "480")?;
        let target_480 = target.join("480");
        link_all_inside(&source_480, &target_480)?;
    } else {
        println!("{}", Yellow.bold().paint(format!("no {}", source_480.to_string_lossy())));
    }
    let source_lo = source.join("lo");
    if source_lo.exists() {
        let target_lo = target.join("lo");
        create_dir_str(target, "lo")?;
        link_all_inside(&source_lo, &target_lo)?;
    } else {
        println!("{}", Yellow.bold().paint(format!("no {}", source_lo.to_string_lossy())));
    }
    Ok(())
}

fn process_music_dir(source: &Path, target: &Path) -> Result<()> {
    // some .mus file at the root (couple dozen bytes each, 40 files or so)
    // one lone .acm file
    // around 40 directories with  some .acm inside
    // create the directories, link the .acm inside
    // copy all the .mus files and link the single .acm in the root
    let music_files = source.read_dir()?;
    for file in music_files {
        let file = file?;
        if file.file_type()?.is_dir() {
            create_dir_os(target, &file.file_name())?;
            link_all_inside(&source.join(&file.file_name()), &target.join(&file.file_name()))?;
        } else {
            let file_path = file.path();
            let extension = file_path.extension().unwrap_or(&*NO_EXT);
            if extension == *MUS_EXT {
                // copy *.mus
                copy_item_os(source, target, &file.file_name())?;
            } else {
                // link the non-dir, non-mus file(s)
                link_item_os(source, target, &file.file_name())?;
            }
        }
    }

    Ok(())
}

fn link_all_inside(source: &Path, target: &Path) -> Result<()> {
    let files = source.read_dir()?;
    for file in files {
        let file = file?;
        link_item_os(source, target, &file.file_name())?;
    }
    Ok(())
}

enum Os {
    Linux,
    Win,
    Mac,
}
enum Vendor {
    Gog,
    Steam,
    Beamdog,
}
enum Variant {
    Classic,
    Ee,
}
struct HintStructure {
    os: Os,
    vendor: Vendor,
    variant: Variant,
}

struct GameDescription {
    os: Os,
    vendor: Vendor,
    variant: Variant,
    name: Option<String>,
    version: Option<String>,
    build: Option<String>,
}

fn check_source(dir: &Path, hint: &HintStructure) -> Result<GameDescription, AnyError> {
    // should have a start.sh script, a game and support
    match hint {
        HintStructure {
            os: Os::Linux,
            vendor: Vendor::Gog,
            variant: Variant::Ee,
        } => check_source_linux_gog_ee(dir, hint),
        _ => Err(anyhow!("don't know yet how to process this variant")),
    }
}

fn check_source_linux_gog_ee(dir: &Path, hint: &HintStructure) -> Result<GameDescription> {
    let start_sh = dir.join("start.sh");
    let game_dir = dir.join("game");
    let support_dir = dir.join("support");
    if !(start_sh.exists() && game_dir.is_dir() && support_dir.is_dir()) {
        return Err(anyhow!("Nope, not a game dir"));
    }

    return Ok(GameDescription {
        os: Os::Linux,
        vendor: Vendor::Gog,
        variant: Variant::Ee,
        name: None,
        version: None,
        build: None,
    });
}
