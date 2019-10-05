#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate cuecard_indexer;
use std::env;

fn main() {
	use clap::App;
	env_logger::init();

	let yml = load_yaml!("cli.yml");
	let matches = App::from_yaml(yml).get_matches();

	let database_option = matches.value_of("database");

	let database_url = match database_option {
		Some(opt) => opt.to_string(),
		_ => env::var("DATABASE_URL").expect("DATABASE_URL must be set")
	};

	let basepath = matches.value_of("INPUT").unwrap();

	let config = cuecard_indexer::Config { basepath: basepath.to_string(), database_url: database_url };

	cuecard_indexer::run(&config);
}