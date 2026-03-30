use calcard::vcard::{VCard, VCardProperty, VCardValue};
use chrono::{Datelike as _, Timelike as _};
use rayon::iter::{ParallelBridge as _, ParallelIterator as _};
use std::num::NonZeroU16;
use std::path::PathBuf;

struct Birthday {
    year: Option<NonZeroU16>,
    month: u8,
    day: u8,
}

fn main() {
    let mut args = std::env::args_os();
    let program_name = args.next().unwrap();
    let input_path = args.next().map(PathBuf::from);
    let output_path = args.next().map(PathBuf::from);
    let Some((input_path, output_path)) = Option::zip(input_path, output_path) else {
        eprintln!("Usage: {} <input> <output>", program_name.display());
        std::process::exit(1);
    };
    if !output_path.exists()
        && let Err(err) = std::fs::create_dir_all(&output_path)
    {
        eprintln!(
            "Error creating output directory {}: {}",
            output_path.display(),
            err
        );
        return;
    }
    let input_dir = match std::fs::read_dir(input_path) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error reading input directory: {}", err);
            std::process::exit(1);
        }
    };
    input_dir.par_bridge().for_each(|input_file| {
        let input_file = match input_file {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Error reading input file: {}", err);
                return;
            }
        };
        let input_path = input_file.path();
        let input_str = match std::fs::read_to_string(&input_path) {
            Ok(str) => str,
            Err(err) => {
                eprintln!("Error reading input file {}: {}", input_path.display(), err);
                return;
            }
        };
        let vcard = match VCard::parse(&input_str) {
            Ok(vcard) => vcard,
            Err(_) => {
                eprintln!("Error parsing input file {}", input_path.display());
                return;
            }
        };

        let contact_fields = vcard
            .entries
            .into_iter()
            .fold((None, None, None), |acc, entry| match entry.name {
                VCardProperty::Uid => {
                    let uid = entry.values.into_iter().next().map(|value| match value {
                        VCardValue::Text(uid) => uid,
                        _ => unreachable!(),
                    });
                    (uid, acc.1, acc.2)
                }
                VCardProperty::Fn => {
                    let name = entry.values.into_iter().next().map(|value| match value {
                        VCardValue::Text(name) => name,
                        _ => unreachable!(),
                    });
                    (acc.0, name, acc.2)
                }
                VCardProperty::Bday => {
                    let birthday = entry
                        .values
                        .into_iter()
                        .next()
                        .map(|value| match value {
                            VCardValue::PartialDateTime(datetime) => datetime,
                            _ => unreachable!(),
                        })
                        .and_then(|datetime| {
                            let mut year = datetime.year.unwrap_or_default();
                            if year < 1700 {
                                year = 0;
                            }
                            let year = NonZeroU16::new(year);
                            let month = datetime.month?;
                            let day = datetime.day?;
                            Some(Birthday { year, month, day })
                        });
                    (acc.0, acc.1, birthday)
                }
                _ => acc,
            });
        let (uid, name, birthday) = match contact_fields {
            (Some(uid), Some(name), Some(birthday)) => (uid, name, birthday),
            _ => return,
        };
        let now = chrono::Utc::now();
        let event_str = format!(
            r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//birthday-calendar//Anniversaire//FR
BEGIN:VEVENT
UID:{}
DTSTAMP:{:04}{:02}{:02}T{:02}{:02}{:02}Z
DTSTART;VALUE=DATE:{:04}{:02}{:02}
RRULE:FREQ=YEARLY
SUMMARY:Anniversaire {}{}
CATEGORIES:BIRTHDAY
END:VEVENT
END:VCALENDAR
"#,
            uid,
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            birthday.year.map(NonZeroU16::get).unwrap_or(2000),
            birthday.month,
            birthday.day,
            name,
            birthday
                .year
                .map(|year| format!(
                    "\nDESCRIPTION:Né le {:02}/{:02}/{:02}",
                    birthday.day,
                    birthday.month,
                    year.get(),
                ))
                .unwrap_or_default(),
        );
        let output_file = output_path.join(format!("{}.ics", uid));
        if let Err(err) = std::fs::write(&output_file, event_str) {
            eprintln!(
                "Error writing output file {}: {}",
                output_file.display(),
                err
            );
        }
    });
}
