extern crate regex;
extern crate chrono;
extern crate libc;
extern crate clap;
extern crate clipboard;
extern crate ncurses;



use regex::{Regex};
use std::collections::HashMap;

use chrono::*;

use libc::system;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

use clap::{Arg};
use clap::App;

use clipboard::ClipboardContext;

use ncurses::*;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::io::SeekFrom;

use std::fmt;

use std::{thread, time};

mod structs;

fn speak(data: &CStr) {
    extern { fn system(data: *const c_char); }

    unsafe { system(data.as_ptr()) }
}

fn ui_header()
{
    printw("Welcome to ACT_linux!\n\n\n");
    refresh();
}

fn ui_update( body: &str)
{
    clear();
    ui_header();
    printw(body);
    refresh();
}

fn main()
{
    let matches = App::new("ACT_linux")
        .version("0.1.0")
        .author("Bergman. <Morindhal@gmail.com>")
        .about("Parses EQ2 logs")
            .arg(Arg::with_name("FILE")
                .help("Sets the log-file to use")
                .required(true)
                .index(1))
            .arg(Arg::with_name("player")
                .required(true)
                .help("Sets the character name to parse, this only catches the YOU and YOUR lines"))
        .get_matches();
    /*Set log-file and player whos view the combat is parsed from based on CL input, player should be replaced with a name collected from the file-string*/
    let from_file = matches.value_of("FILE").unwrap();
    let player = matches.value_of("player").unwrap();
    let f = File::open(from_file).unwrap();

    let mut ctx = ClipboardContext::new().unwrap();

    /*start the n-curses UI*/
    initscr();
    ui_header();
    //getch();

    let mut encounters: Vec<structs::Encounter> = Vec::new();
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>\D*?)(' |'s |YOUR |YOU )(?P<attack>\D*)(((multi attack)|hits|hit|flurry|(aoe attack))|(( multi attacks)| hits| hit)) (?P<target>\D+) (?P<crittype>\D+) (?P<damage>\d+) (?P<damagetype>[A-Za-z]+) damage").unwrap();
    let timeparser = Regex::new(r"(?P<day_week>[A-Za-z]+) (?P<month>[A-Za-z]+)  (?P<day_month>\d+) (?P<hour>\d+):(?P<minute>\d+):(?P<second>\d+) (?P<year>\d+)").unwrap();

    let mut file = BufReader::new(&f);
    /*jump to the end of the file, negative value here will go to the nth character before the end of file.. Positive values are not encouraged.*/
    file.seek(SeekFrom::End(0));
    
    let mut buffer = String::new();
    let mut battle_timer = time::Instant::now();
    let mut ui_update_timer = time::Instant::now();
    let mut fightdone = true;
    'main: loop
    {
        buffer.clear();
        if file.read_line(&mut buffer).unwrap() > 0
        {
            /*Spawn a seperate thread to deal with the triggers*/
            let triggerbuffer = buffer.clone();
            thread::spawn( move || 
            {
                /*The container for the triggers, the key is what the tts should say, the value is the regex that is matched.*/
                let mut triggers: HashMap<&str, Regex> = HashMap::new();
                    triggers.insert("Ruling I am", Regex::new(r".*I rule.*").unwrap());
                    triggers.insert("Verily", Regex::new(r".*i also rule.*").unwrap());
                for (trigger, trigged) in triggers.iter()
                {
                    match trigged.captures(triggerbuffer.as_str()) {None => {}, Some(cap) =>
                    {
                        speak(&CString::new(format!("espeak \"{}\"", trigger)).unwrap());
                    }};
                }
            });
            match re.captures(buffer.as_str()) {None => {}, Some(cap) =>
            {
                match timeparser.captures(cap.name("datetime").unwrap()) {None => {}, Some(time_cap) =>
                {
                    let parsed_time = UTC
                                            .ymd(
                                                time_cap.name("year").unwrap().parse::<i32>().unwrap(),
                                                match time_cap.name("month").unwrap() {"Jan"=>0, "Feb"=>1, "Mar"=>2, "Apr"=>3,  "May"=>4, "Jun"=>5, "Jul"=>6, "Aug"=>7, "Sep"=>8, "Oct"=>9, "Nov"=>10, "Dec"=>11, _=>0},
                                                time_cap.name("day_month").unwrap().parse::<u32>().unwrap())
                                            .and_hms(
                                                time_cap.name("hour").unwrap().parse::<u32>().unwrap(),
                                                time_cap.name("minute").unwrap().parse::<u32>().unwrap(),
                                                time_cap.name("second").unwrap().parse::<u32>().unwrap()
                                                );
                if fightdone
                {
                    printw("\n\n\n\n\n");
                    encounters.push(structs::Encounter{ attackers: Vec::new(), encounter_start: parsed_time, encounter_end: parsed_time, encounter_duration : 0, player : String::from(player.clone()) });
                    fightdone = false;
                }
                    encounters.last_mut().unwrap().attack(cap);
                    encounters.last_mut().unwrap().encounter_duration = 0;// encounter_duration is not currently used, will probably be when history parsing gets done
                    encounters.last_mut().unwrap().encounter_end = parsed_time; //assume every line ends the encounter, likely not optimal, needs to be overhauled
                    battle_timer = time::Instant::now();
                }};
                }
            };
        }
        else /*Sleep for 0.1 sec if nothing has happened in the log-file*/
        {
            thread::sleep(time::Duration::from_millis(100));
        }
        /*update the UI, only once every 1 sec*/
        if !fightdone && ui_update_timer.elapsed() >= time::Duration::from_millis(1000)
        {
            ui_update_timer = time::Instant::now();
            encounters.last_mut().unwrap().attackers.sort();
            ui_update(&*format!("{}{}", encounters[match encounters.len() {val => if val == 0 || val == 1 {0} else {val-2}}], encounters.last().unwrap()));
        }
        /*End current encounter if nothing has been parsed in combat within the last 3 secs*/
        if battle_timer.elapsed() >= time::Duration::from_millis(3000)
        {
            if !fightdone
            {
                encounters.last_mut().unwrap().attackers.sort();
                ui_update(&*format!("{}{}", encounters[match encounters.len() {val => if val == 0 || val == 1 {0} else {val-2}}], encounters.last().unwrap()));
                match ctx.set_contents(format!("{}", encounters.last().unwrap()))
                {
                    Ok(_)=>
                    {
                        /*This is currently linux dependant, probably not the best idea for future alerts but for now it "works" assuming one has the correct file on the system*/
                        speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                    },
                    Err(e)=>{println!("Clipboard error: {}", e);}
                }
                fightdone = true;
            }
        }
    }
}

/*
(1477272172)[Mon Oct 24 03:22:52 2016] YOUR Raging Whirlwind multi attacks Bonesnapper for a critical of 38996 cold damage.\r\n
(1477272172)[Mon Oct 24 03:22:52 2016] Kabernet\'s Asylum multi attacks Bonesnapper for a critical of 36622 mental damage.\r\n
(1478123041)[Wed Nov  2 22:44:01 2016] YOU hit training dummy for a critical of 377262 heat damage.
(1478123041)[Wed Nov  2 22:44:01 2016] YOU multi attack training dummy for a critical of 148320 heat damage.
(1478132824)[Thu Nov  3 01:27:04 2016] Devorstator's Impatience heals Devorstator for 43947 hit points.
YOU aoe attack training dummy for a critical of 204585 heat damage.

println!("{}",buffer) <-- add this to the match X.captures() statement in the None body. Also needs to disable the code in the ui_update function.

*/
