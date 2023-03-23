use byteorder::{LittleEndian, ReadBytesExt};
use datetime::*;
use std::io;
use std::mem::transmute;
use std::time::Duration;
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
        iter_list_with_mru(extension);
    }

    Ok(())
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
            "{}\t\t| {}\t\t| {:x?}",
            mru_position
                .iter()
                .position(|x| x.to_string() == name)
                .unwrap(),
            name,
            value.bytes
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

pub fn bin_to_systemtime(bin_value: Vec<u8>) {
    let mut year: &[u8] = &[bin_value[0], bin_value[1]];
    let year: u16 = year.read_u16::<LittleEndian>().unwrap();
    let mut month: &[u8] = &[bin_value[2], bin_value[3]];
    let month: u16 = month.read_u16::<LittleEndian>().unwrap();
    let mut day_of_week: &[u8] = &[bin_value[4], bin_value[5]];
    let num = day_of_week.read_u16::<LittleEndian>().unwrap();
    let mut day: &[u8] = &[bin_value[6], bin_value[7]];
    let day: u16 = day.read_u16::<LittleEndian>().unwrap();
    let mut hour: &[u8] = &[bin_value[8], bin_value[9]];
    let hour: u16 = hour.read_u16::<LittleEndian>().unwrap();
    let mut minute: &[u8] = &[bin_value[10], bin_value[11]];
    let minute: u16 = minute.read_u16::<LittleEndian>().unwrap();
    let mut second: &[u8] = &[bin_value[12], bin_value[13]];
    let second: u16 = second.read_u16::<LittleEndian>().unwrap();

    print!("{:2}.{:2}.{:2} ", day, month, year);
    match num {
        0 => print!("Sun "),
        1 => print!("Mon "),
        2 => print!("Tue "),
        3 => print!("Wed "),
        4 => print!("Thu "),
        5 => print!("Fri "),
        6 => print!("Sat "),
        _ => println!(""),
    }
    println!("{:2}:{:2}:{:2} ", hour, minute, second);
}

pub fn rawvalue_to_timestamp(tmp: Vec<u8>) -> LocalDateTime {
    let bytes_to_nanos = u64::from_le_bytes(tmp.try_into().unwrap()) * 100;
    let nanos_to_secs: i64 = Duration::from_nanos(bytes_to_nanos)
        .as_secs()
        .try_into()
        .unwrap();
    let windows_base_date = LocalDate::ymd(1601, Month::January, 1).unwrap();
    let hour: i8 = 0;
    let minute: i8 = 0;
    let windows_base_time = LocalTime::hm(hour, minute).unwrap();
    let windows_base_timestamp = LocalDateTime::new(windows_base_date, windows_base_time);
    windows_base_timestamp.add_seconds(nanos_to_secs)
}

pub fn split_iso_timestamp<'a>(iso_timestamp: LocalDateTime) -> String {
    let mut string_vec: Vec<String> = Vec::new();
    iso_timestamp
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
