
use std::{fs, io::{self, Read, Write}, path::Path};
use serde_json::{Value};
use tar::{Builder, Archive};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

  //            //
 // CONSTANTES //
//            //
const IGNORE_FILE: &str = ".mvcignore";
const STANDARD_IGNORE: &str = 
"target
.mvc
foo.tar
.mvcignore";
const SNAP_METADATA_PATH: &str = ".mvc/metadata";
const SNAP_ARCHIVE_PATH: &str = ".mvc/archives";

  //            //
 // STRUCTURES //
//            //
#[derive(Serialize, Deserialize)]
struct Snapshot {
    hash: String,
    message: String
}
  //           //
 // FUNCTIONS //
//           //
/// вычисляет хеш файла
fn calculate_hash(path: &str) -> Result<String, std::io::Error>  {
    let mut file = fs::File::open(path)?; // открываем файл
    let mut hasher = Sha256::new(); // создаем хешер
    io::copy(&mut file, &mut hasher)?; // копируем контент из file в hasher
    Ok(format!("{:x}", hasher.finalize())) // возвращаеем форматируя как строку и заканчивая хеш
}
/// проверяет, надо ли игнорировать путь
fn should_ignore(path: &Path, ignore_list: &[&str]) -> bool{
    if !path.is_absolute() { // если путь не абсолютный
        let ancestors = path.ancestors(); // Создает итератор по объекту Path и его предкам.
        
        for ancestor in ancestors { // берем все элементы
            if &ancestor == &Path::new(".") {return true;} // если путь это . (текущая директория) то игнорируй
            // let ancestor = ancestor.strip_prefix("./").unwrap();
            if ignore_list.iter().any(|ignore| {
                //println!("{} == {}: {}", *ignore, ancestor.as_os_str().to_str().unwrap(), Some(*ignore) == ancestor.as_os_str().to_str());
                let ignore_path = Some(*ignore) == ancestor.as_os_str().to_str();// если в игнор листе будет наш путь
                return ignore_path;  
            }){
                return true; // то возвращаем true (да, игнорировать)
            }
        }
    } else {
        return true; // если путь будет абсолютным то есть риск удалить системные файлы, поэтому лучше будет игнорить
    }
    return false; // ну а если вапще чета как та и не то и не другое то false
}
/// Проверяет в репозитории ли мы
fn is_in_repo() -> Result<bool, io::Error> {
    let folder = fs::exists(".mvc");
    folder
}
/// Получение ignorelist'а
fn get_ignore() -> Result<String, io::Error> {
    if !is_in_repo()? {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in repo"));
    }
    let ignore = fs::read_to_string(IGNORE_FILE);
    ignore
}
/// удаляет все то что не в игнор листе
fn delete_current(path: &Path) -> Result<(), std::io::Error>  {
    let current_dir = walkdir::WalkDir::new(path).min_depth(1).contents_first(true); // читаем текущую директорию
    let ignore = get_ignore(); // получаем ignorelist
    let ignore: String = ignore?;
    let splited_ignore: Vec<&str> = ignore.split("\n").collect(); //разделяем на строки
    
    for entry in current_dir {
        
        let borrowed_entry = entry?; 
        let should_ignore = !should_ignore(borrowed_entry.path().strip_prefix("./").unwrap(), &splited_ignore);
        //println!("DELETECURRENT y: {}: {}", y.path().strip_prefix("./").unwrap().display(), z);
        if should_ignore {
            if borrowed_entry.metadata().unwrap().is_dir() {
                delete_current(borrowed_entry.path())?;
            } else {
                fs::remove_file(borrowed_entry.path())?;
            }
        }
    }
    Ok(())
}
/// Функция создающая архив с кодом
fn create_archive(file: fs::File) -> Result<(), std::io::Error> {
    let ignore = get_ignore(); // получаем ignorelist
    let ignore = ignore?;
    let splited_ignore: Vec<&str> = ignore.split("\n").collect(); //разделяем на строки
    // let mut objects: Vec<String> = vec![];
    let mut archive = Builder::new(file); // Создаем архив в файле
    let current_dir = fs::read_dir("."); // читаем текущую строку
    for object in current_dir.unwrap() {
        let object = object?; // берем объект
        let object_name = object.file_name(); // берем имя в виде строки
        if !splited_ignore.iter().any(|f| *f==object_name.as_os_str()) { // если строка ignore != файл то
            if object.metadata().unwrap().is_dir() { //проверь директория ли это
                archive.append_dir_all(&object_name, &object_name).unwrap(); // и заархивируй ее с дочерними элементами
            }
            else { 
                archive.append_path(object_name).unwrap(); // просто заархивируй
            }
        }
    }
    Ok(())
}
/// функция создающая архив с json (снапшот)
fn create_snap(snap_id: u32, message: &str) -> Result<(), std::io::Error> {
    create_archive(fs::File::create(format!("{}/{}.tar", SNAP_ARCHIVE_PATH, snap_id))?)?;
    let hash = calculate_hash(&format!("{}/{}.tar", SNAP_ARCHIVE_PATH, snap_id))?;
    let snapshot = Snapshot{
        hash: hash.to_string(),
        message: message.to_string()
    };

    let json_format: String = serde_json::to_string(&snapshot).unwrap();
    let mut info = fs::File::create(format!("{}/{}.json", SNAP_METADATA_PATH, snap_id))?;
    info.write(json_format.as_bytes())?;
    Ok(())
}
/// Иницилизация нового репозитория
fn init() -> Result<(), std::io::Error> {
    if is_in_repo()? {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "The repository has already been initialized."));
    } else {
        fs::create_dir_all(SNAP_ARCHIVE_PATH)?;  // } создаем папки
        fs::create_dir_all(SNAP_METADATA_PATH)?; // }
        let mut ignore = fs::File::create(IGNORE_FILE)?; // создаем ignore list
        ignore.write(STANDARD_IGNORE.as_bytes())?; // записываем его
        // create_snap(1 , "Initial")?; // создаем снапшот
        let mut head = fs::File::create(".mvc/HEAD")?;
        head.write("0".as_bytes())?;
        println!("Repository initialized! Please execute \"mvc save Initial\" for create first commit!");
    }
    Ok(())
}
/// распаковка архива с данными по id
fn unpack_arch(id: &u32) -> Result<(), std::io::Error> {
    delete_current(&Path::new("."))?;
    let path = format!("{}/{}.tar", SNAP_ARCHIVE_PATH, id);
    let file = fs::File::open(path)?;
    let mut archive = Archive::new(file);
    archive.unpack(".")?;
    Ok(())
}
/// Функция для возврата к снапшоту
fn return_to_snap(id: u32) -> Result<(), std::io::Error> {
    if !is_in_repo()? {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in repo"));
    }
    let new_hash = calculate_hash(&format!("{}/{}.tar", SNAP_ARCHIVE_PATH, id))?;
    // получаем его метаданные
    let mut metadata: String = String::new();
    let mut metadata_file = fs::File::open(format!("{}/{}.json", SNAP_METADATA_PATH, id))?;
    metadata_file.read_to_string(&mut metadata)?;
    let metadata: Value = serde_json::from_str(&metadata)?;
    if metadata["hash"].as_str().unwrap() != new_hash {
        return Err(std::io::Error::new(io::ErrorKind::Other, "Hashs not match"))
    }
    // распаковываем архив...
    unpack_arch(&id)?;
    // выводим всю инфу:
    println!("Message: {}", metadata["message"]);
    Ok(())
}
/// Функция для того чтобы цифарки обновить снапшота и тд
fn save_snap(message: &str) -> Result<(), std::io::Error> {
    if !is_in_repo()? {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in repo"));
    }
    let last_snap_str = fs::read_to_string(".mvc/HEAD")?;
    let last_snap_int: u32 = last_snap_str.parse().unwrap_or_else(|e| {
        eprintln!("[{}] cant str -> int, {}", "ERROR".red(),e);
        std::process::exit(1);
    });
    let last_snap_int = last_snap_int + 1;
    
    create_snap(last_snap_int, message)?;
    fs::write(".mvc/HEAD", last_snap_int.to_string())?;
    println!("Saved!");
    Ok(())
}
/// Вывод всех снапшотов с их инфой
fn read_all_snaps() -> Result<(), std::io::Error> {
    if !is_in_repo()? {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in repo"));
    }
    let mut paths: Vec<_> = fs::read_dir(SNAP_METADATA_PATH).unwrap()
        .map(|r| r.unwrap())
        .collect();
    paths.sort_by_key(|path| path.path());

    for path in paths {
        let file_name = path.file_name();
        let file = fs::read_to_string(path.path())?;
        let file = file.as_str();
        let value: Value = serde_json::from_str(file)?;
        let pretty_name = file_name.to_str().unwrap_or("ERROR").replace(".json", "");
        println!("{}: {}
{}:        {}
Message:     {}
----", "Snapshot ID".bright_cyan(), pretty_name,"Hash".bright_purple(), value["hash"], value["message"]);
        }
    
    Ok(())
}
/// основная функция.
fn run() -> Result<(), std::io::Error>  {
    
    let args: Vec<String> = std::env::args().collect();
    let version = || {println!("{} v{}. Licensed under MIT license", env!("CARGO_PKG_NAME").bright_green(), env!("CARGO_PKG_VERSION"))};
    let usage = || {println!(
"Usage:
    mvc [-v | --version] <command> [<args>]
Commands:
    init              - initialize a new repository
    log               - display all snapshots
    return <id>       - returns to <id> version
    save <message>    - saves version
    help              - show this message")};
    if args.len() == 2 {
        if args[1] == "init" {
            init()?;
        } else if args[1] == "log" {
            read_all_snaps()?;
        } else if args[1] == "--version" || args[1] == "-v"{
            version()
        } else{usage()}
    } else if args.len() >= 3 {
        if args[1] == "return" {
            return_to_snap(args[2].parse().unwrap())?;
        } else if args[1] == "save" {
            save_snap(&args[2..].join(" "))?;   
        } else {usage()} 
    } else {usage()}
    Ok(())
}
fn main() {
    if let Err(e) = run() { // если Err(e) [(e это io::Error)] будет равен вызванной функции run()
        eprintln!("[{}] {}","ERROR".red(), e); // то выведи e (ошибку)
        std::process::exit(1); // и заверши выполнение с кодом 1
    }
}