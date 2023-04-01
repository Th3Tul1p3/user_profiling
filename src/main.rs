use datetime::*;
use std::io;
use std::mem::transmute;
use std::time::Duration;
use winapi::um::minwinbase::SYSTEMTIME;
use windows::{Win32::System::Com::*, Win32::UI::Shell::Common::*, Win32::UI::Shell::*};
use winreg::enums::*;
use winreg::EnumValues;
use winreg::RegKey;
use winreg::RegValue;

fn main() -> io::Result<()> {
    println!("--------------- User profiling ---------------\n");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // get searches accomplished in Explorer
    println!("----- searches accomplished in Explorer ------");
    let word_wheel_query =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\WordWheelQuery")?;

    println!("MRU position\t| Number\t| Value\n----------------------------------------------");
    iter_list_with_mru(word_wheel_query);

    // get recent doc with timestamps when opened
    println!("\n\n---------------- Recents docs ----------------");
    let recent_docs =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\RecentDocs")?;
    for sub_key in recent_docs.enum_keys().map(|x| x.unwrap()) {
        println!("File type : {}", sub_key);
        let extension = recent_docs.open_subkey(sub_key)?;
        iter_list_with_mru_rd(extension);
        println!("");
    }

    // evidence of file save
    println!("\n\n----------------- Saved file -----------------");
    let comdlg32 = hkcu.open_subkey(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\ComDlg32\\OpenSavePidlMRU",
    )?;
    for sub_key in comdlg32.enum_keys().map(|x| x.unwrap()) {
        println!("File type : {}", sub_key);
        let extension = comdlg32.open_subkey(sub_key)?;
        iter_list_with_mru_sf(extension);
        println!("");
    }

    // evidence of typed path
    println!("\n\n----------------- Typed path -----------------");
    let typed_paths =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\TypedPaths")?;
    for (name, value) in typed_paths.enum_values().map(|x| x.unwrap()) {
        println!("{name}: {}", value.to_string());
    }

    // evidence of office file
    println!("\n\n--------------- Office evidence --------------");
    // TODO gestion d'erreur si pas de version 16.0
    let office = hkcu.open_subkey("Software\\Microsoft\\Office\\16.0")?;
    // enumerate different office (Work, excel, Onenote....)
    for office_product in office.enum_keys().map(|x| x.unwrap()) {
        if ["Word", "Excel", "OneNote", "PowerPoint"]
            .iter()
            .any(|&s| s == office_product)
        {
            println!("{office_product}");
            let office_product_key =
                match office.open_subkey(format!("{}\\User MRU", office_product)) {
                    Ok(regkey) => regkey,
                    Err(_) => {
                        eprintln!("Office product does not have an User MRU");
                        continue;
                    }
                };

            // enumerate each live ID
            // TODO Affichage FolderID nécessaire ?
            for live_id in office_product_key.enum_keys().map(|x| x.unwrap()) {
                println!("Live ID: {}\nFile MRU:", live_id);
                // For File MRU
                let file_mru = office_product_key.open_subkey(format!("{}\\File MRU", live_id))?;
                iter_list_with_office_mru(file_mru);

                println!("\nPlace MRU:");
                // For Place MRU
                let place_mru =
                    office_product_key.open_subkey(format!("{}\\Place MRU", live_id))?;
                iter_list_with_office_mru(place_mru);
                println!("");
            }
        }
    }
    Ok(())
}

pub fn iter_list_with_office_mru(regkey: RegKey) {
    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name.contains("FOLDERID") {
            continue;
        }

        let mut start_timestamp: usize = value.to_string().find("T").unwrap();
        let timestamp = value.to_string()[start_timestamp + 1..start_timestamp + 17].to_string();
        let timestamp_u64 = u64::from_str_radix(&timestamp, 16).unwrap();
        start_timestamp = value.to_string().find("*").unwrap();
        print!("MRU Time: {}\t MRU location: ", rawvalue_to_timestamp(timestamp_u64));
        println!(
            "{}",
            value.to_string()[start_timestamp + 1..value.to_string().len() - 1].to_string()
        );
    }
}

pub fn iter_list_with_mru_sf(regkey: RegKey) {
    print_systemtime(regkey.query_info().unwrap().get_last_write_time_system());

    // find MRUListEx to get order of usage
    let mru_position = get_mru_order(regkey.enum_values());

    unsafe {
        let mut _com = CoInitialize(None).unwrap();
        println!("MRU position\t| Number\t| Value\n----------------------------------------------");
        for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
            if name == "MRUListEx" {
                continue;
            }

            // resolve this PIDL's absolute path
            let mut buffer: Vec<u8> = value.bytes;
            let other_item: IShellItem =
                match SHCreateItemFromIDList(buffer.as_mut_ptr() as *mut ITEMIDLIST) {
                    Ok(ishellitem) => ishellitem,
                    Err(message) => {
                        eprintln!("{}", message);
                        continue;
                    }
                };
            let other_name = match other_item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING) {
                Ok(display_name) => display_name.to_string().unwrap(),
                Err(message) => {
                    eprintln!("{}", message);
                    continue;
                }
            };
            println!(
                "{}\t\t| {}\t\t| {}",
                mru_position
                    .iter()
                    .position(|x| x.to_string() == name)
                    .unwrap(),
                name,
                other_name
            );
        }
        _com = CoUninitialize();
    }
}

pub fn iter_list_with_mru_rd(regkey: RegKey) {
    print_systemtime(regkey.query_info().unwrap().get_last_write_time_system());

    // find MRUListEx to get order of usage
    let mru_position = get_mru_order(regkey.enum_values());

    println!("MRU position\t| Number\t| Value\n----------------------------------------------");
    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            continue;
        }

        // convert the value in array of u32
        let mut byte_array: Vec<u16> = Vec::new();
        for x in (0..value.bytes.len()).step_by(2) {
            let a: [u8; 2] = value.bytes[x..x + 2].try_into().unwrap();
            byte_array.push(unsafe { transmute::<[u8; 2], u16>(a) }.to_le())
        }

        // the value contain filename_utf16, filname.lnk_utf8, filename.lnk_utf16
        // We take the first and the last value and we clean it
        let split_array = byte_array.split_at(byte_array.iter().position(|x| *x == 0u16).unwrap());
        let first_string_array = split_array.0;
        let second_string_start = split_array
            .1
            .iter()
            .position(|&r| r == *first_string_array.get(0).unwrap())
            .unwrap();
        let mut second_string = split_array.1.split_at(second_string_start).1;
        second_string = second_string
            .split_at(second_string.iter().position(|x| *x == 0u16).unwrap())
            .0;

        println!(
            "{}\t\t| {}\t\t| {}, {}",
            mru_position
                .iter()
                .position(|x| x.to_string() == name)
                .unwrap(),
            name,
            String::from_utf16(first_string_array).unwrap(),
            String::from_utf16(second_string).unwrap()
        );
    }
}

pub fn iter_list_with_mru(regkey: RegKey) {
    print_systemtime(regkey.query_info().unwrap().get_last_write_time_system());

    // find MRUListEx to get order of usage
    let mru_position = get_mru_order(regkey.enum_values());

    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            continue;
        }
        println!(
            "{}\t\t| {}\t\t| {}",
            mru_position
                .iter()
                .position(|x| x.to_string() == name)
                .unwrap(),
            name,
            String::from_utf8(value.bytes).unwrap(),
        );
    }
}

pub fn u8_array_to_u32_vec(value: RegValue) -> Vec<u32> {
    // convert order of searche from u8 to u32
    let mut mru_position = Vec::new();
    let mut counter: usize = 0;
    let mut tmp_array: [u8; 4] = [0u8; 4];
    for bytes in value.bytes.clone() {
        tmp_array[counter % 4] = bytes;
        counter += 1;
        if counter % 4 == 0 {
            // last number is always max int u32
            if unsafe { transmute::<[u8; 4], u32>(tmp_array) }.to_le() != u32::MAX {
                mru_position.push(unsafe { transmute::<[u8; 4], u32>(tmp_array) }.to_le());
            }
        }
    }
    mru_position
}

pub fn print_systemtime(system_time_val: SYSTEMTIME) {
    print!("Last write : ");
    print!(
        "{}.{}.{} ",
        system_time_val.wDay, system_time_val.wMonth, system_time_val.wYear
    );
    match system_time_val.wDayOfWeek {
        0 => print!("Sun "),
        1 => print!("Mon "),
        2 => print!("Tue "),
        3 => print!("Wed "),
        4 => print!("Thu "),
        5 => print!("Fri "),
        6 => print!("Sat "),
        _ => println!(""),
    }
    println!(
        "{}:{}:{} ",
        system_time_val.wHour, system_time_val.wMinute, system_time_val.wSecond
    );
}

pub fn rawvalue_to_timestamp(tmp: u64) -> String {
    let nanos_to_secs: i64 = Duration::from_nanos(tmp * 100)
        .as_secs()
        .try_into()
        .unwrap();
    let windows_base_date = LocalDate::ymd(1601, Month::January, 1).unwrap();
    let windows_base_time = LocalTime::hm(0i8, 0i8).unwrap();
    let windows_base_timestamp = LocalDateTime::new(windows_base_date, windows_base_time);
    let mut string_vec: Vec<String> = Vec::new();
    windows_base_timestamp
        .add_seconds(nanos_to_secs)
        .iso()
        .to_string()
        .split("T")
        .for_each(|x| string_vec.push(x.to_string()));
    format!(
        "{} {}",
        string_vec.get(0).unwrap(),
        string_vec.get(1).unwrap()
    )
}

pub fn get_mru_order(mru_values: EnumValues<'_>) -> Vec<u32> {
    // find MRUListEx to get order of usage
    let mut mru_position = Vec::new();
    for (name, value) in mru_values.map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            mru_position = u8_array_to_u32_vec(value);
            break;
        }
    }
    mru_position
}