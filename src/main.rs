use expanduser::expanduser;
use std::collections::BTreeMap;
use std::env;
use std::env::args;
use std::fs::create_dir;
use std::fs::remove_dir_all;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

fn main() -> Result<()> {
    let arguments: Vec<_> = args().collect();
    let glob_str = if arguments.len() > 1 {
        let mut s = arguments[1].trim();
        if s.starts_with('"') && s.ends_with('"') {
            s = s.strip_prefix('"').unwrap();
            s = s.strip_suffix('"').unwrap();
        }
        s
    } else {
        panic!("Please provide a glob path that contains the photos you want to look at");
    };

    let expanded = {
        let path = expanduser(glob_str)?;
        if Path::new(&path).is_relative() {
            let mut pwd = env::current_dir()?;
            pwd.push(path);
            pwd
        } else {
            path
        }
    };
    let expanded_str = expanded.to_str().expect("Could not parse provided path");

    let _ = remove_dir_all("./output");
    create_dir("./output")?;
    let mut times: BTreeMap<NaiveDate, Vec<(NaiveTime, String)>> = BTreeMap::new();
    for path in glob::glob(expanded_str)? {
        let path = path?;
        let file_path = path.display().to_string();
        let file = std::fs::File::open(path)?;
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = exifreader.read_from_container(&mut bufreader);
        let mut parsed = NaiveDateTime::from_timestamp_millis(0).unwrap();
        let mut date = parsed.date();
        let mut time = parsed.time();
        if let Ok(e) = exif {
            if let Some(f) = e.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                let dt = f.display_value().with_unit(&e).to_string();
                parsed = NaiveDateTime::parse_from_str(&dt, "%Y-%m-%d %H:%M:%S").unwrap();
                date = parsed.date();
                time = parsed.time();
            }
        }
        times.entry(date).or_default().push((time, file_path));
    }

    let mut html_index = 1;
    let mut picture_count = 0;
    let mut prev_index = 0;

    let mut f = File::create(format!("output/{}.html", html_index))?;
    let mut bf = BufWriter::new(f);

    bf.write_all(b"<head><script src='./main.js'></script></head>")?;

    for (date, file_names) in times.into_iter().rev() {
        let s = format!("<h2>{}</h2>\n", date);
        let bytes = s.as_bytes();
        bf.write_all(bytes)?;
        bf.write_all(b"<div>\n")?;
        for (_, file_name) in file_names.into_iter().rev() {
            picture_count += 1;
            let s = format!(
                "<img loading='lazy' width='33%' src='{}' alt='{}'></img>\n",
                file_name, file_name
            );
            let bytes = s.as_bytes();
            bf.write_all(bytes)?;
        }
        bf.write_all(b"</div>\n")?;
        if picture_count / 25 > prev_index {
            prev_index = picture_count / 25;
            html_index += 1;

            f = File::create(format!("output/{}.html", html_index))?;
            bf = BufWriter::new(f);
            bf.write_all(b"<head><script src='./main.js'></script></head>")?;
        }
    }

    let js_file = format!(
        r#"let href = window.location;
let split = href.pathname.split('/');
let lastIndex = split.length - 1;
let filename = split[lastIndex];

let splitPath = filename.split('.');
let num = parseInt(splitPath[0]);
let prevIndex = num - 1;
let nextIndex = num + 1;
let prev = `${{prevIndex}}.html`;
let next = `${{nextIndex}}.html`;

let prevPath = [...split];
let nextPath = [...split];
prevPath[lastIndex] = prev;
nextPath[lastIndex] = next;

prevPath = prevPath.join('/');
nextPath = nextPath.join('/');

console.log(prevPath, nextPath);

window.addEventListener(
  "keydown",
  (event) => {{
    if (event.code == 'ArrowLeft' && prevIndex > 0) {{
      window.location.href = prevPath;
    }} else if (event.code == 'ArrowRight' && nextIndex < {html_index}) {{
      window.location.href = nextPath;
    }}
  }},
);
"#
    );

    let f = File::create("output/main.js")?;
    let mut bf = BufWriter::new(f);

    bf.write_all(js_file.as_bytes())?;

    Ok(())
}
