#![feature(insert_str)]

extern crate regex;
extern crate chrono;
extern crate libc;
extern crate clap;
extern crate clipboard;
extern crate ncurses;

use std::mem;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, RecvTimeoutError};

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

static ENCOUNTER_WINDOW_WIDTH: i32 = 30;

fn speak(data: &CStr) {
    extern { fn system(data: *const c_char); }

    unsafe { system(data.as_ptr()) }
}


fn ui_update( body: &str, highlight: &str, pointer: &(i32,i32,bool,i32,bool,i32), encounters: &Vec<structs::Encounter>, current_encounter: &structs::Encounter)
{
    let mut max_x = 0;
    let mut max_y = 0;
    getmaxyx(stdscr(), &mut max_y, &mut max_x);


    let mut main_win = newwin(max_y-22, max_x-ENCOUNTER_WINDOW_WIDTH, 20,ENCOUNTER_WINDOW_WIDTH);
    let mut header_win = newwin(20, max_x, 0, 0);
    let mut encounter_win = newwin(max_y-22, ENCOUNTER_WINDOW_WIDTH, 20, 0);

    wclear(main_win);
    wclear(header_win);
    wclear(encounter_win);

    
    wmove(header_win, 1, 1);
    wprintw(header_win, " Welcome to ACT_linux!\n\n\n\tESC to exit.\n\tc to copy the last completed fight to the clipboard.\n\tC to copy the current fight to the clipboard.\n\tEnter/Backspace to toggle a lock of the encounter-view to what is selected (X) or move to the newest encounter at each update.\n\n");

    let mut draw: String;
    if !encounters.is_empty() && pointer.0 as usize >= 0 && encounters.len() > pointer.0 as usize
    {
        if pointer.1 == 0
        {
            draw = format!(" {:?}\n", encounters[pointer.0 as usize]);
        }
        else if pointer.1 == 1 && pointer.4
        {
            draw = format!(" {} attacks:\n {}\n", encounters[pointer.0 as usize].attackers[pointer.3 as usize].name, encounters[pointer.0 as usize].attackers[pointer.3 as usize]);
        }
        else if pointer.1 == 1 && !pointer.4
        {
            draw = format!(" {:?}\n", encounters[pointer.0 as usize]);
        }
        else
        {
            draw = format!("{}.", body);
        }
    }
    else
    {
        draw = String::from(body);
    }
    wmove(main_win, 1, 1);
    wattron(main_win, A_BOLD());
    wprintw(main_win, "\tEncounters:\n\n");
    wattroff(main_win, A_BOLD());
    for line in draw.lines()
    {
        if line.contains(highlight)
        {
            wattron(main_win, COLOR_PAIR(1));
            wprintw(main_win, &if pointer.2 {format!(" [ ]{}\n", line)} else {format!("    {}\n", line)});
            wattroff(main_win, COLOR_PAIR(1));
        }
        else
        {
            wprintw(main_win, &if pointer.2 {format!(" [ ]{}\n", line)} else {format!("    {}\n", line)});
        }
    }
    
    for i in 0..encounters.len()
    {
        mvwprintw(encounter_win, i as i32 + 1, 1, &format!("[ ]Duration: {}:{}\n", encounters[i].encounter_duration/60, encounters[i].encounter_duration % 60 ));
    }
    wattron(encounter_win, COLOR_PAIR(1));
    mvwprintw(encounter_win, encounters.len() as i32 + 1, 1, &format!("[ ]Duration: {}:{}\n", current_encounter.encounter_duration/60, current_encounter.encounter_duration % 60 ));
    wattroff(encounter_win, COLOR_PAIR(1));

    wborder(main_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(header_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(encounter_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);

    wrefresh(main_win);
    wrefresh(header_win);
    wrefresh(encounter_win);


    wmove(encounter_win, 1+pointer.0, 2);
    if pointer.2
    {
        waddch(encounter_win, 'X' as chtype);
        wmove(encounter_win, 1+pointer.0, 2);
    }
    wrefresh(encounter_win);
    if pointer.1 == 1
    {
        //inspect encounter, mark individual attackers
        wmove(main_win, 4+pointer.3, 2);
        if pointer.4
        {
            waddch(main_win, 'X' as chtype);
            wmove(main_win, 4+pointer.5, 2);
        }
        wrefresh(main_win);
    }

    delwin(main_win);
    delwin(header_win);
    delwin(encounter_win);
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
    let player_display = String::from(player);
    let f = File::open(from_file).unwrap();
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>\D*?)(' |'s |YOUR |YOU | )(?P<attack>\D*)(((multi attack)|hits|hit|flurry|(aoe attack)|flurries|(multi attacks)|(aoe attacks))|(( multi attacks)| hits| hit)) (?P<target>\D+) (?P<crittype>\D+) (?P<damage>\d+) (?P<damagetype>[A-Za-z]+) damage").unwrap();
    let timeparser = Regex::new(r"(?P<day_week>[A-Za-z]+) (?P<month>[A-Za-z]+)  (?P<day_month>\d+) (?P<hour>\d+):(?P<minute>\d+):(?P<second>\d+) (?P<year>\d+)").unwrap();
    let mut file = BufReader::new(&f);
    /*jump to the end of the file, negative value here will go to the nth character before the end of file.. Positive values are not encouraged.*/
    file.seek(SeekFrom::End(0));



    /*start the n-curses UI*/
    initscr();
    keypad(stdscr(), true);
    noecho();
    start_color();
    init_pair(1, COLOR_RED, COLOR_BLACK);


    let (parse_tx, main_rx) = mpsc::channel::<Box<(bool,structs::Encounter)>>();
    let (user_tx, mainss_rx) = mpsc::channel();

    let mut buffer = String::new();
    let mut battle_timer = time::Instant::now();
    let mut ui_update_timer = time::Instant::now();
    let mut fightdone = true;
    let dummy_time = UTC.ymd(2016, 2, 3).and_hms(0, 0, 0);
    
    
    let buttonlistener = thread::spawn(move || 
    {
        'input: loop/*Listen to input, send input to main*/
        {
            user_tx.send(getch()).unwrap();
        }
    });
    

    let ui = thread::spawn(move ||
    {
        let mut ctx = ClipboardContext::new().unwrap();
        let timeout = time::Duration::from_millis(10);
        let mut pointer: (i32, i32, bool, i32, bool, i32) = (0, 0, false, 0, false, 0);
        let mut encounters: Vec<structs::Encounter> = Vec::new();
        let mut current_encounter: structs::Encounter = structs::Encounter{attackers: Vec::new(), encounter_start: dummy_time, encounter_end: dummy_time, encounter_duration : 0, player : String::from(player_display.clone()) };
        ui_update("", &player_display, &pointer, &encounters, &current_encounter);
        'ui: loop
        {
            match main_rx.recv_timeout(timeout)
            {
                Ok(val) => 
                {
                    
                    if val.0
                    {
                        encounters.push(current_encounter.clone());
                    }
                    if !pointer.2
                    {
                        pointer.0 = encounters.len() as i32;
                    }
                    current_encounter = val.1;
                    ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                },
                Err(e) => {}
            }
            match mainss_rx.recv_timeout(timeout)
            {
                Ok(val) => match val
                    {
                        27 => // escape
                        {
                            endwin();
                            std::process::exit(1);
                        },
                        99 =>  // C
                            match ctx.set_contents(format!("{}", current_encounter))
                            {
                                Ok(_)=>
                                {
                                    /*This is currently linux dependant, probably not the best idea for future alerts but for now it "works" assuming one has the correct file on the system*/
                                    speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                                },
                                Err(e)=>{println!("Clipboard error: {}", e);}
                            },
                        67 => // c
                            match ctx.set_contents(format!("{}", encounters.last().unwrap()))
                            {
                                Ok(_)=>
                                {
                                    /*This is currently linux dependant, probably not the best idea for future alerts but for now it "works" assuming one has the correct file on the system*/
                                    speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                                },
                                Err(e)=>{println!("Clipboard error: {}", e);}
                            },
                        KEY_UP => 
                        {
                            if pointer.1 == 0 && !pointer.4
                            {
                                if pointer.0>0
                                {
                                    pointer.0-=1;
                                }
                            }
                            else if pointer.1 == 1 && !pointer.4
                            {
                                if pointer.3>0
                                {
                                    pointer.3-=1;
                                }
                            }
                            else if pointer.4
                            {
                                if pointer.5>0
                                {
                                    pointer.5-=1;
                                }
                            }
                            ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                        },
                        KEY_DOWN => 
                        {
                            if pointer.1 == 0
                            {
                                if pointer.0 < encounters.len() as i32
                                {
                                    pointer.0+=1;
                                }
                            }
                            else if pointer.1 == 1 && !encounters.is_empty() && pointer.3 < encounters[pointer.0 as usize].attackers.len() as i32 - 1 && !pointer.4
                            {
                                pointer.3+=1
                            }
                            else if pointer.4
                            {
                                pointer.5+=1;
                            }
                                ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                        },
                        KEY_LEFT => 
                        {
                            if pointer.1 == 1 && pointer.2 && !pointer.4
                            {
                                pointer.1 = 0;
                                ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                            }
                        },
                        KEY_RIGHT => 
                        {
                            if pointer.1 == 0 && pointer.2 && pointer.0 < encounters.len() as i32
                            {
                                pointer.1 = 1;
                                pointer.3 = 0;
                                ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                            }
                        },
                        10 => // enter
                        {
                            if pointer.2 && pointer.1 == 1
                            {
                                pointer.4 = true;
                                pointer.5 = 0;
                            }
                            pointer.2=true;
                            ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                        },
                        KEY_BACKSPACE =>
                        {
                            if pointer.4
                            {
                                pointer.4 = false;
                            }
                            else
                            {
                                pointer.2 = false;
                                pointer.1 = 0;
                            }
                            ui_update(&format!("{:?}", current_encounter), &player_display, &pointer, &encounters, &current_encounter);
                        },
                        _ => {}//ui_update(&format!("{}", encounters[0].attackers.len()), &player_display, &pointer, &encounters, &current_encounter);}//ui_update(&format!("{}", val), &player_display, &pointer, &encounters, &current_encounter);}
                    },
                Err(e) => {}
            }
        }
    });

    let mut encounter_counter: u64 = 0;
    let mut encounter: structs::Encounter = structs::Encounter{attackers: Vec::new(), encounter_start: dummy_time, encounter_end: dummy_time, encounter_duration : 0, player : String::from(player.clone()) };
    'parser: loop/*Parse file, send results to main every X secs*/
    {
        'encounter_loop: loop
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
                match re.captures(buffer.as_str()) {None => {/*println!("{}",buffer)*/}, Some(cap) =>
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
                            encounter = structs::Encounter{ attackers: Vec::new(), encounter_start: parsed_time, encounter_end: parsed_time, encounter_duration : 0, player : String::from(player.clone()) };
                            fightdone = false;
                        }
                        encounter.attack(cap);
                        encounter.encounter_end = parsed_time; //assume every line ends the encounter, likely not optimal, needs to be overhauled
                        encounter.encounter_duration = (encounter.encounter_end-encounter.encounter_start).num_seconds() as u64;
                        battle_timer = time::Instant::now();
                    }};
                }};
            }
            else /*Sleep for 0.1 sec if nothing has happened in the log-file*/
            {
                thread::sleep(time::Duration::from_millis(100));
            }
            /*update the UI, once every 1 sec*/
            if !fightdone && ui_update_timer.elapsed() >= time::Duration::from_millis(1000)
            {
                ui_update_timer = time::Instant::now();
                encounter.attackers.sort();
                parse_tx.send(Box::new((false, encounter.clone())));
            }
            /*End current encounter if nothing has been parsed in combat within the last 3 secs*/
            if battle_timer.elapsed() >= time::Duration::from_millis(3000)
            {
                if !fightdone
                {
                    encounter.attackers.sort();
                    parse_tx.send(Box::new((true, encounter.clone())));
                    encounter_counter += 1;
                    fightdone = true;
                    break 'encounter_loop;
                }
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
