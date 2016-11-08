#![feature(insert_str)]

extern crate regex;
extern crate chrono;
extern crate libc;
extern crate clap;
extern crate clipboard;
extern crate ncurses;

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


fn ui_update( body: &str, highlight: &str, pointer: (i32,i32))
{
    let tvec = vec!["Blubb", "blubbing", "Blibbiest!"];
    let mut max_x = 0;
    let mut max_y = 0;
    getmaxyx(stdscr(), &mut max_y, &mut max_x);

    //let mut start_y = (max_y - WINDOW_HEIGHT) / 2;            
    //let mut start_x = (max_x - WINDOW_WIDTH) / 2;
    //let mut win = create_win(start_y, start_x);


    let mut main_win = newwin(max_y-22, max_x-ENCOUNTER_WINDOW_WIDTH, 20,ENCOUNTER_WINDOW_WIDTH);
    let mut header_win = newwin(20, max_x, 0, 0);
    let mut encounter_win = newwin(max_y-22, ENCOUNTER_WINDOW_WIDTH, 20, 0);

    wclear(main_win);
    wclear(header_win);
    wclear(encounter_win);

    //wborder(main_win, '|' as chtype, '-' as chtype, '_' as chtype, '|' as chtype, '|' as chtype, '|' as chtype, '|' as chtype, '|' as chtype);
    //printw(format!(""));
    wmove(header_win, 1, 1);
    wprintw(header_win, " Welcome to ACT_linux!\n\n\n\tESC to exit.\n\tc to copy the last completed fight to the clipboard.\n\tC to copy the current fight to the clipboard.\n\n");

    wmove(main_win, 1, 1);
    attron(A_BOLD());
    wprintw(main_win, "\tEncounters:\n\n");
    attroff(A_BOLD());
    for line in body.lines()
    {
        if line.contains(highlight)
        {
            attron(COLOR_PAIR(1));
            wprintw(main_win, &format!(" [ ]{}", line));
            attroff(COLOR_PAIR(1));
        }
        else
        {
            wprintw(main_win, &format!(" [ ]{}", line));
        }
        wprintw(main_win, "\n");
    }
    
    for i in 0..tvec.len()
    {
        mvwprintw(encounter_win, i as i32 + 1, 1, &format!("[ ]{}", tvec[i]));
    }

    wborder(main_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(header_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(encounter_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);

    wrefresh(main_win);
    wrefresh(header_win);
    wrefresh(encounter_win);

    if pointer.1 == 0
    {
        wmove(encounter_win, 1+pointer.0, 2);
        waddch(encounter_win, 'X' as chtype);
        wmove(encounter_win, 1+pointer.0, 2);
        wrefresh(encounter_win);
    }
    else if pointer.1 == 1
    {
        //inspect encounter, mark individual attackers
        wmove(main_win, 3+pointer.0, 2);
        waddch(main_win, 'X' as chtype);
        wmove(main_win, 3+pointer.0, 2);
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
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>\D*?)(' |'s |YOUR |YOU )(?P<attack>\D*)(((multi attack)|hits|hit|flurry|(aoe attack))|(( multi attacks)| hits| hit)) (?P<target>\D+) (?P<crittype>\D+) (?P<damage>\d+) (?P<damagetype>[A-Za-z]+) damage").unwrap();
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
    
    ui_update(" ", player, (0,0));

    //getch();

    let mut encounterss = Arc::new(Mutex::new(Vec::new()));
    let (parse_tx, main_rx) = mpsc::channel::<Box<(u64,String)>>();
    let (user_tx, mainss_rx) = mpsc::channel();

    let mut buffer = String::new();
    let mut battle_timer = time::Instant::now();
    let mut ui_update_timer = time::Instant::now();
    let mut fightdone = true;
    
    
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
        let mut before_last = String::from("");
        let mut last_fight = String::from("");
        let mut encounter_counter: u64 = 0;
        let mut pointer: (i32, i32) = (0, 0);
        'ui: loop
        {
            match main_rx.recv_timeout(timeout)
            {
                Ok(val) => 
                {
                    if encounter_counter < val.0
                    {
                        encounter_counter += 1;
                        before_last = last_fight;
                    }
                    last_fight = val.1;
                    ui_update(&format!("{}{}", before_last, last_fight), player_display.as_str(), (0,0));
                },
                Err(e) => {}
            }
            match mainss_rx.recv_timeout(timeout)
            {
                Ok(val) => match val //ui_update(format!("{}",val).as_str())    <-- to find specific keys
                    {
                        27 => {endwin();std::process::exit(1);},
                        99 => 
                            match ctx.set_contents(format!("{}", before_last))
                            {
                                Ok(_)=>
                                {
                                    /*This is currently linux dependant, probably not the best idea for future alerts but for now it "works" assuming one has the correct file on the system*/
                                    speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                                },
                                Err(e)=>{println!("Clipboard error: {}", e);}
                            },
                        67 => 
                            match ctx.set_contents(format!("{}", last_fight))
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
                            if pointer.0>0
                            {
                                pointer.0-=1;
                                ui_update(&format!("{}{}", before_last, last_fight), player_display.as_str(), pointer);
                            }
                        },
                        KEY_DOWN => 
                        {//if pointer.0 < encounters.len()
                            pointer.0+=1;
                            ui_update(&format!("{}{}", before_last, last_fight), player_display.as_str(), pointer);
                        },
                        KEY_LEFT => 
                        {
                            if pointer.1 == 1
                            {
                                pointer.1 = 0;
                                ui_update(&format!("{}{}", before_last, last_fight), player_display.as_str(), pointer);
                            }
                        },
                        KEY_RIGHT => 
                        {
                            if pointer.1 == 0
                            {
                                pointer.1 = 1;
                                ui_update(&format!("{}{}", before_last, last_fight), player_display.as_str(), pointer);
                            }
                        },
                        _ => {}
                    },
                Err(e) => {}
            }
        }
    });

    let mut encounter_counter: u64 = 0;
    let mut encounters = encounterss.lock().unwrap();
    'parser: loop/*Parse file, send results to main every X secs*/
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
            parse_tx.send(Box::new((encounter_counter, format!("{:?}", encounters.last().unwrap()))));
            //ui_update(&*format!("{}{}", encounters[match encounters.len() {val => if val == 0 || val == 1 {0} else {val-2}}], encounters.last().unwrap()));
        }
        /*End current encounter if nothing has been parsed in combat within the last 3 secs*/
        if battle_timer.elapsed() >= time::Duration::from_millis(3000)
        {
            if !fightdone
            {
                encounters.last_mut().unwrap().attackers.sort();
                parse_tx.send(Box::new((encounter_counter, format!("{:?}", encounters.last().unwrap()))));
                encounter_counter += 1;
                //ui_update(&*format!("{}{}", encounters[match encounters.len() {val => if val == 0 || val == 1 {0} else {val-2}}], encounters.last().unwrap()));
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
