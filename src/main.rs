use std::io;
use std::mem::transmute;
use winapi::um::minwinbase::SYSTEMTIME;
use winreg::enums::*;
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
           // break;
           // convert PIDL to path.... must find a way 
        
        }
    /*    // evidence of typed path
        let _typed_paths =
            hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\TypedPaths")?;
    */
    Ok(())
}

pub fn iter_list_with_mru_sf(regkey: RegKey) {
    let raw_last_write_time = regkey.query_info().unwrap().get_last_write_time_system();
    print!("Last write : ");
    print_systemtime(raw_last_write_time);

    // find MRUListEx to get order of usage
    let mut mru_position = Vec::new();
    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            mru_position = u8_array_to_u32_vec(value);
            break;
        }
    }
    println!("MRU position\t| Number\t| Value\n----------------------------------------------");
    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            continue;
        }
        // convert the value in array of u32 
        let mut byte_array: Vec<u16> = Vec::new();
        for x in (0..value.bytes.len()-2).step_by(2) {
            let a: [u8; 2] = value.bytes[x..x + 2].try_into().unwrap();
            byte_array.push(unsafe { transmute::<[u8; 2], u16>(a) }.to_le())
        }
        let split_array = byte_array.split_at(16).0;
        println!(
            "{}\t\t| {}\t\t| {:x?}",
            mru_position
                .iter()
                .position(|x| x.to_string() == name)
                .unwrap(),
            name,
            split_array
        );
    }
}

pub fn iter_list_with_mru_rd(regkey: RegKey) {
    let raw_last_write_time = regkey.query_info().unwrap().get_last_write_time_system();
    print!("Last write : ");
    print_systemtime(raw_last_write_time);

    // find MRUListEx to get order of usage
    let mut mru_position = Vec::new();
    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            mru_position = u8_array_to_u32_vec(value);
            break;
        }
    }
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
        second_string = second_string.split_at(second_string.iter().position(|x| *x == 0u16).unwrap()).0;
        
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
    let raw_last_write_time = regkey.query_info().unwrap().get_last_write_time_system();
    print!("Last write : ");
    print_systemtime(raw_last_write_time);

    // find MRUListEx to get order of usage
    let mut mru_position = Vec::new();
    for (name, value) in regkey.enum_values().map(|x| x.unwrap()) {
        if name == "MRUListEx" {
            mru_position = u8_array_to_u32_vec(value);
            break;
        }
    }

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
    print!(
        "{:2}.{:2}.{:2} ",
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
        "{:2}:{:2}:{:2} ",
        system_time_val.wHour, system_time_val.wMinute, system_time_val.wSecond
    );
}