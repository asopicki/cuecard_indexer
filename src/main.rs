#![feature(fs_read_write)]
extern crate diesel;
extern crate walkdir;
extern crate regex;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate uuid as uuidcrate;
extern crate cuer_database;

use self::cuer_database::*;
use self::models::*;
use self::diesel::prelude::*;
use self::walkdir::{WalkDir, DirEntry};
use regex::Regex;
use uuidcrate::Uuid;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

struct IndexFile<> {
	path: DirEntry,
	content: String,
	meta: HashMap<String, String>,
}

impl IndexFile {
	fn set_content(&mut self, content: &str) {
		self.content = content.to_string();
	}

	fn set_meta(&mut self, key: &str, value: &str) {
		self.meta.insert(String::from(key), String::from(value));
	}

	fn get_meta(&self, key: String) -> Option<&String> {
		return self.meta.get(&key);
	}

	fn index_file(&self, file: &IndexFile) -> Option<PathBuf>  {
		let mut filename = ".de.sopicki.cuelib.".to_string();
		filename.push_str(file.path.file_name().to_str().unwrap());
		filename = filename.to_string();
		let path = Path::new(&filename).to_owned();
		let parent = file.path.path().parent().unwrap();
		return Some(parent.join(&path));
	}
}

fn is_allowed(entry: &DirEntry) -> bool {
	let filename = entry.file_name().to_str().unwrap().to_lowercase();

	if filename.ends_with(".md") && !filename.starts_with(".de.sopicki.cuelib") {
		return true
	}

	return false
}

fn process(entry: DirEntry) -> IndexFile {
	lazy_static! {
		static ref TITLE_PATTERN: Regex = Regex::new(r"^#\s+(?P<title>.*)$").unwrap();
		static ref META_PATTERN: Regex = Regex::new(r"^[\*]\s+[\*][\*](?P<metaname>\w+)[\*][\*]:\s+(?P<metatext>.*)$").unwrap();
		static ref PHASE_PATTERN: Regex = Regex::new(r"^(I|II|III|IV|V|VI)\s*(\+.*)?$").unwrap();
	}

	let filename = entry.path().to_str().unwrap().to_owned();
	let content = std::fs::read_string(entry.path()).unwrap();
	let mut index_file = IndexFile { path: entry, content: "".to_owned(), meta: HashMap::new() };
	let mut has_title = false;

	for line in content.lines() {
		if !has_title {
			let result = TITLE_PATTERN.captures(line);
			match result {
				Some(caps) => {
					index_file.set_meta("title", caps.name("title").unwrap().as_str());
					has_title = true;
				},
				_ => ()
			}
		}

		let result = META_PATTERN.captures(line);
		match result {
			Some(caps) => {
				let key = caps.name("metaname").unwrap().as_str();
				index_file.set_meta(&key.to_lowercase(), caps.name("metatext").unwrap().as_str());
			},
			_ => ()
		}
	}
	index_file.set_content(&content);

	let default = "unphased".to_string();

	let phase = index_file.get_meta("phase".to_string()).unwrap_or(&default).clone();
	let result = PHASE_PATTERN.captures(&phase);

	match result {
		Some(caps) => {
			let p = match caps.get(1) {
				Some(m) => m.as_str(),
				_ => "unphased"
			};

			let plusfigures = match caps.get(2) {
				Some(m) => m.as_str(),
				_ => ""
			};

			index_file.set_meta("phase", p);
			index_file.set_meta("plusfigures", plusfigures);
		}
		_ => {
			index_file.set_meta("phase", "unphased");
			index_file.set_meta("plusfigures", "");
		}
	}

	index_file.set_meta("_cuesheetpath", &filename);

	return index_file;
}

fn index(connection: &SqliteConnection, file: &IndexFile) {
	let u = Uuid::new_v4();
	let unphased = "unphased".to_string();
	let unknown = "unknown".to_string();
	let empty = "".to_string();

	let values = NewCuecard {
		uuid: &u.hyphenated().to_string(),
		phase: file.get_meta("phase".to_string()).unwrap_or(&unphased),
		rhythm: file.get_meta("rhythm".to_string()).unwrap_or(&unknown),
		title: file.get_meta("title".to_string()).unwrap_or(&unknown),
		choreographer: file.get_meta("choreographer".to_string()).unwrap_or(&unknown),
		steplevel: file.get_meta("steplevel".to_string()).unwrap_or(&empty),
		difficulty: file.get_meta("difficulty".to_string()).unwrap_or(&empty),
		meta: &serde_json::to_string(&file.meta).unwrap_or("{}".to_string()),
		content: &file.content
	};
	values.create_or_update(connection).unwrap();

	let index_file = file.index_file(file).unwrap();

	std::fs::write(index_file, u.hyphenated().to_string()).unwrap();
}

fn update(connection: &SqliteConnection, file: &IndexFile) {
	let unphased = "unphased".to_string();
	let unknown = "unknown".to_string();
	let empty = "".to_string();

	let indexfile = file.index_file(file).unwrap();
	let fileuuid = std::fs::read_string(indexfile).unwrap();

	let values = NewCuecard {
		uuid: &fileuuid,
		phase: file.get_meta("phase".to_string()).unwrap_or(&unphased),
		rhythm: file.get_meta("rhythm".to_string()).unwrap_or(&unknown),
		title: file.get_meta("title".to_string()).unwrap_or(&unknown),
		choreographer: file.get_meta("choreographer".to_string()).unwrap_or(&unknown),
		steplevel: file.get_meta("steplevel".to_string()).unwrap_or(&empty),
		difficulty: file.get_meta("difficulty".to_string()).unwrap_or(&empty),
		meta: &serde_json::to_string(&file.meta).unwrap_or("{}".to_string()),
		content: &file.content
	};

	values.create_or_update(connection).unwrap();
}

enum IndexAction {
	Index,
	Update,
	NotModified
}



fn should_index(connection: &SqliteConnection, file: &IndexFile) -> IndexAction {
	use self::schema::cuecards::dsl::*;
	let indexfile = file.index_file(file).unwrap();
	if indexfile.exists() {
		let modified = file.path.path().metadata().unwrap().modified().unwrap();
		let imodified = indexfile.metadata().unwrap().modified().unwrap();
		if modified > imodified {
			return IndexAction::Update;
		} else {
			let fileuuid = std::fs::read_string(indexfile).unwrap();
			let result = cuecards.filter(uuid.eq(fileuuid)).load::<Cuecard>(connection).unwrap();
			if result.is_empty() {
				return IndexAction::Index;
			}
		}

		return IndexAction::NotModified;
	}

	return IndexAction::Index;
}

fn main() {
	env_logger::init().unwrap();

	let walkdir = WalkDir::new("/home/alex/Round")
		.min_depth(2).into_iter().filter_map(|e| e.ok())
		.filter(|e|is_allowed(e));

	let mut files: Vec<IndexFile> = vec![];

	for entry in  walkdir {
		debug!("{}", entry.path().display());
		let indexfile = process(entry);
		files.push(indexfile);
	}

	let connection = establish_connection();
	for file in files {
		match should_index(&connection, &file) {
			IndexAction::Update => {
				info!("Reindexing file: {:?}", file.path.path().file_name());
				update(&connection, &file);
			},
			IndexAction::Index => {
				info!("Indexing new file: {:?}", file.path.path().file_name());
				index(&connection, &file);
			},
			_ => {
				debug!("File not modified: {:?}", file.path.path().file_name());
			}
		}
	}

}