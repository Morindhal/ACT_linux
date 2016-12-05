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


fn ui_update( body: &str, highlight: &str, ui_data: &mut structs::ui_data, encounters: &mut Vec<structs::CombatantList>)
{
    let mut max_x = 0;
    let mut max_y = 0;
    getmaxyx(stdscr(), &mut max_y, &mut max_x);
//    max_y = 5;
    ui_data.nav_main_win_scroll.0 = max_y - 22;
    ui_data.nav_encounter_win_scroll.0 = max_y - 22;
//-22

    let mut main_win = newwin(ui_data.nav_main_win_scroll.0, max_x-ENCOUNTER_WINDOW_WIDTH, 20,ENCOUNTER_WINDOW_WIDTH);
    let mut header_win = newwin(20, max_x, 0, 0);
    let mut encounter_win = newwin(ui_data.nav_encounter_win_scroll.0, ENCOUNTER_WINDOW_WIDTH, 20, 0);

    wclear(main_win);
    wclear(header_win);
    wclear(encounter_win);

    
    wmove(header_win, 1, 1);
    wprintw(header_win, " Welcome to ACT_linux!\n\n\n\tESC to exit.\n\tc to copy the last completed fight to the clipboard.\n\tC to copy the current fight to the clipboard.\n\tTAB to toggle a lock of the encounter-view to what is selected (X) or move to the newest encounter at each update.\n\t+ to begin editing the filters used to only  show certain attacks when inspecting a player.\n\n");
    wprintw(header_win, " Filters: ");
    wprintw(header_win, &ui_data.filters);

    let mut draw = String::from("");
    if !ui_data.debug
    {
        if !ui_data.is_locked() //render normally, navigating left side
        {
            encounters[ui_data.nav_xy[0].0 as usize - ui_data.nav_encounter_win_scroll.1 as usize].combatants.sort();
            draw = format!("{:?}\n", encounters[ui_data.nav_xy[0].0 as usize - ui_data.nav_encounter_win_scroll.1 as usize]);
        }
        else if ui_data.nav_lock_encounter //render normally, navigation right side
        {
            encounters[ui_data.nav_xy[0].0 as usize - ui_data.nav_encounter_win_scroll.1 as usize].combatants.sort();
            draw = format!("{:?}\n", encounters[ui_data.nav_xy[0].0 as usize - ui_data.nav_encounter_win_scroll.1 as usize]);
        }
        else if ui_data.nav_lock_combatant //replace right side with the combatants attacks
        {
            encounters[ui_data.nav_xy[0].0 as usize].combatants.sort();
            if encounters[ui_data.nav_xy[0].0 as usize].combatants.len() != 0
            {
                /*draw = format!("{} attacks:\n{}\n", 
                    encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name,
                    encounters[ui_data.nav_xy[0].0 as usize].print_attacks(&ui_data.filters, (&encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name)));*/
                draw = format!("{} attacks:\n{}\n", 
                    encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name,
                    encounters[ui_data.nav_xy[0].0 as usize].print_attack_stats(&encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name.as_str()));
            }
            else
            {
                draw = format!("{}.", body);
            }
        }
        else
        {
            draw = format!("{}.", body);
        }
    }
    else
    {
        draw = format!("{}.", body);
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
            wprintw(main_win, &if ui_data.nav_lock_encounter {format!(" [ ]{}\n", line)} else {format!("    {}\n", line)});
            wattroff(main_win, COLOR_PAIR(1));
        }
        else
        {
            wprintw(main_win, &if ui_data.nav_lock_encounter {format!(" [ ]{}\n", line)} else {format!("    {}\n", line)});
        }
    }
    
    if !encounters.is_empty()
    {
        let mut bound:usize = 0;
        if (bound as i32 - (ui_data.nav_encounter_win_scroll.0 - 2)) < 0 {bound = 0;}
        else { bound = encounters.len() - (ui_data.nav_encounter_win_scroll.0 as usize - 2); }
        let mut line_print = 1;

        for i in (bound - ui_data.nav_encounter_win_scroll.1 as usize)..(encounters.len() - 1 - ui_data.nav_encounter_win_scroll.1 as usize)
        {// från X till encounter längd 0-3 1-4 2-5 minus scroll, 0-3 1-4-1=0-3 osv
            //if encounters.len() <= i - ui_data.nav_encounter_win_scroll.1 as usize || (i as i32 - ui_data.nav_encounter_win_scroll.1 < 0) {break;}
            mvwprintw(encounter_win, line_print, 1, &format!("[ ]Duration: {}:{:02}\n", encounters[i - ui_data.nav_encounter_win_scroll.1 as usize].encounter_duration/60, encounters[i - ui_data.nav_encounter_win_scroll.1 as usize].encounter_duration % 60 ));
            line_print += 1;
        }
        if ui_data.nav_encounter_win_scroll.1 == 0
        {
            wattron(encounter_win, COLOR_PAIR(1));
            mvwprintw(encounter_win, line_print, 1, &format!("[ ]Duration: {}:{:02}\n", encounters.last().unwrap().encounter_duration/60, encounters.last().unwrap().encounter_duration % 60 ));
            wattroff(encounter_win, COLOR_PAIR(1));
        }
    }

    wborder(main_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(header_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(encounter_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);

    wrefresh(main_win);
    wrefresh(header_win);
    wrefresh(encounter_win);


    wmove(encounter_win, 1+ if ui_data.nav_xy[0].0 >= (ui_data.nav_encounter_win_scroll.0 - 2) { ui_data.nav_encounter_win_scroll.0 - 3 } else { ui_data.nav_xy[0].0 } , 2);
    if ui_data.nav_lock_refresh
    {
        waddch(encounter_win, 'O' as chtype);
    }
    else
    {
        waddch(encounter_win, 'X' as chtype);
    }
    wmove(encounter_win, 1+ if ui_data.nav_xy[0].0 >= (ui_data.nav_encounter_win_scroll.0 - 2) { ui_data.nav_encounter_win_scroll.0 - 3 } else { ui_data.nav_xy[0].0 } , 2);
    wrefresh(encounter_win);

    if ui_data.nav_lock_encounter
    {
        //inspect encounter, mark individual attackers
        wmove(main_win, 4+ui_data.nav_xy[1].0, 2);
        waddch(main_win, 'X' as chtype);
        wmove(main_win, 4+ui_data.nav_xy[1].0, 2);
        wrefresh(main_win);
    }
    else if ui_data.nav_lock_combatant
    {
        wmove(main_win, 1+ui_data.nav_xy[2].0, 2);
        waddch(main_win, 'X' as chtype);
        wmove(main_win, 1+ui_data.nav_xy[2].0, 2);
        wrefresh(main_win);
    }
    
    if ui_data.nav_lock_filter
    {
        wmove(header_win, 10, 10+ui_data.filters.len() as i32);
        wrefresh(header_win);
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
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>\D*?)(' |'s |YOUR |YOU | )(?P<attack>\D*)(((multi attack)|hits|hit|flurry|(aoe attack)|flurries|(multi attacks)|(aoe attacks))|(( multi attacks)| hits| hit)) (?P<target>\D+) for(?P<crittype>\D*)( of | )(?P<damage>\d+) (?P<damagetype>[A-Za-z]+) damage").unwrap();
    let mut file = BufReader::new(&f);
    /*jump to the end of the file, negative value here will go to the nth character before the end of file.. Positive values are not encouraged.*/
    file.seek(SeekFrom::End(0));



    /*start the n-curses UI*/
    initscr();
    keypad(stdscr(), true);
    noecho();
    start_color();
    init_pair(1, COLOR_RED, COLOR_BLACK);


    let (parse_tx, main_rx) = mpsc::channel::<Box<(bool,Vec<structs::Attack>)>>();
    let (user_tx, mainss_rx) = mpsc::channel();
    //let (timer_tx, timer_rx) = mpsc::channel();

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
    
/*    let timer_counter = thread::spawn(move || 
    {
        let timeout = time::Duration::from_millis(500);
        let mut time = time::Instant::now();
        let mut timers: HashMap<&str, i32> = HashMap::new();
        'timer: loop/*Listen to timers, give tts warning when time is up*/
        {
            match timer_rx.recv_timeout()
            {
                Ok(val) =>
                {
                    //add countdown timer
                    timers.insert(val);
                },
                Err(e) => {}
            }
            for (text, time) in timers
            {
                if time <= 0
                {
                    time -= time.elapsed().as_secs();
                    speak(&CString::new(format!("espeak \"{}\"", text)).unwrap());
                    timers.remove(&text);
                }
            }
        }
    });*/


    let ui = thread::spawn(move ||
    {
        let mut ctx = ClipboardContext::new().unwrap();
        let timeout = time::Duration::from_millis(1);
        let mut ui_data = structs::ui_data{nav_xy: vec![(0,0)], nav_lock_encounter: false, nav_lock_combatant: false, nav_lock_filter: false, nav_lock_refresh: true, nav_main_win_scroll: (0, 0), nav_encounter_win_scroll: (5, 0), filters: String::from(""), debug: false};
        let mut encounters: Vec<structs::CombatantList> = Vec::new();
        encounters.push(structs::CombatantList::new(structs::getTime("default_time")));
        let mut update_ui = true;
        'ui: loop
        {
            match main_rx.recv_timeout(timeout)
            {
                Ok(val) => 
                {
                    if val.0
                    {
                        encounters.push(structs::CombatantList::new(structs::getTime("default_time")));
                    }
                    if !ui_data.nav_lock_encounter && ui_data.nav_lock_refresh
                    {
                        if encounters.last().unwrap().attacks.len() != 0
                        {
                            ui_data.nav_xy[0].0 = encounters.len() as i32 - 1;
                            ui_data.nav_encounter_win_scroll.1 = 0;
                        }
                    }
                    for attack in val.1
                    {
                        encounters.last_mut().unwrap().attack(attack);
                    }
                    if encounters.len() != 0
                    {encounters.last_mut().unwrap().encounter_duration = (encounters.last().unwrap().encounter_end-encounters.last().unwrap().encounter_start).num_seconds() as u64;}
                    update_ui = true;
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
                        99 | 67 =>  // C | c
                        {
                            if !ui_data.is_locked()
                            {
                                match ctx.set_contents(format!("{}", encounters[ui_data.nav_xy[0].0 as usize - ui_data.nav_encounter_win_scroll.1 as usize]))//if ui_data.nav_xy[0].0 >= encounters.len() as i32 {&encounters[encounters.len()-1 as usize]} else {&encounters[ui_data.nav_xy[0].0 as usize]}))
                                {
                                    Ok(_)=>
                                    {
                                        /*This is currently linux dependant, probably not the best idea for future alerts but for now it "works" assuming one has the correct file on the system*/
                                        speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                                    },
                                    Err(e)=>{println!("Clipboard error: {}", e);}
                                };
                            }
                            else
                            {
                                ui_data.filters.push( val as u8 as char );
                                update_ui = true;
                            }
                        },
                        KEY_UP => 
                        {
                            if ui_data.nav_xy.last().unwrap().0 > 0
                            {
                                if !ui_data.is_locked()
                                {
                                    if  ui_data.nav_encounter_win_scroll.0 - 2 < encounters.len() as i32
                                    {
                                        if ui_data.nav_encounter_win_scroll.1 < encounters.len() as i32 - ui_data.nav_encounter_win_scroll.0 - 2
                                        {
                                            ui_data.nav_encounter_win_scroll.1 += 1;
                                        }
                                        else
                                        {
                                            ui_data.nav_xy.last_mut().unwrap().0 -= 1;
                                        }
                                    }
                                    else
                                    {
                                        ui_data.nav_xy.last_mut().unwrap().0 -= 1;
                                    }
                                }
                                else if ui_data.nav_lock_encounter && encounters[ui_data.nav_xy[0].0 as usize].combatants.len() as i32 - ui_data.nav_main_win_scroll.0 - ui_data.nav_main_win_scroll.1 < 0 ||
                                        ui_data.nav_lock_encounter && encounters[ui_data.nav_xy[0].0 as usize].combatants.len() as i32 - ui_data.nav_main_win_scroll.0 - ui_data.nav_main_win_scroll.1 < 0
                                {
                                    ui_data.nav_main_win_scroll.1 += 1;speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                                }
                                else
                                {
                                    ui_data.nav_xy.last_mut().unwrap().0 -= 1;
                                }
                            }
                            update_ui = true;
                        },
                        KEY_DOWN => 
                        {
                            if ui_data.nav_lock_encounter && ui_data.nav_xy[1].0 < encounters[ui_data.nav_xy[0].0 as usize].combatants.len() as i32 - 1
                            {
                                ui_data.nav_xy.last_mut().unwrap().0 += 1;
                            }
                            else if ui_data.nav_lock_combatant
                            {
                                ui_data.nav_xy.last_mut().unwrap().0 += 1;
                            }
                            else if !ui_data.is_locked() && ui_data.nav_xy.last().unwrap().0 < encounters.len() as i32 - 1
                            {
                                if ui_data.nav_encounter_win_scroll.1 > 0
                                {
                                    ui_data.nav_encounter_win_scroll.1 -= 1;
                                }
                                else
                                {
                                    ui_data.nav_xy.last_mut().unwrap().0 += 1;
                                }
                            }
                            update_ui = true;
                        },/*
                        KEY_LEFT => 
                        {
                            if !ui_data.nav_lock_filter
                            {
                                if ui_data.nav_xy.last().unwrap().1 == 1 && ui_data.nav_lock_encounter && !ui_data.nav_lock_combatant
                                {
                                    ui_data.nav_xy.last().unwrap().1 = 0;
                                    update_ui = true;
                                }
                            }
                        },
                        KEY_RIGHT => 
                        {
                            if !ui_data.nav_lock_filter
                            {
                                if ui_data.nav_lock_encounter && ui_data.nav_xy.last().unwrap().0 < encounters.len() as i32
                                {
                                    ui_data.deeper();
                                    ui_data.unlock();
                                    ui_data.nav_lock_combatant = true;
                                    update_ui = true;
                                }
                            }
                        },*/
                        10 => // enter
                        {
                            if ui_data.nav_lock_filter
                            {
                                ui_data.surface();
                                ui_data.nav_lock_filter = false;
                            }
                            else if ui_data.nav_lock_encounter
                            {
                                ui_data.deeper();
                                ui_data.nav_lock_encounter = false;
                                ui_data.nav_lock_combatant = true;
                            }
                            else if !ui_data.is_locked()
                            {
                                if ui_data.nav_xy.last().unwrap().0 == encounters.len() as i32
                                {
                                    ui_data.nav_xy.last_mut().unwrap().0 -= 1; // ugly code, will bug around --- needs fix
                                }
                                ui_data.deeper();
                                ui_data.nav_lock_encounter = true;
                            }
                            update_ui = true;
                        },
                        KEY_BACKSPACE =>
                        {
                            if !ui_data.nav_lock_filter
                            {
                                if ui_data.nav_lock_encounter
                                {
                                    ui_data.surface();
                                    ui_data.nav_lock_encounter = false;
                                }
                                else if ui_data.nav_lock_combatant
                                {
                                    ui_data.surface();
                                    ui_data.nav_lock_encounter = true;
                                    ui_data.nav_lock_combatant = false;
                                }
                                else  //backspace with ui_data.nav_xy having a len() of 1
                                {
                                }
                            }
                            else
                            {
                                ui_data.filters.pop();
                            }
                            update_ui = true;
                        },
                        43 => // + key
                        {
                            if !ui_data.nav_lock_filter
                            {
                                ui_data.deeper();
                                ui_data.nav_lock_filter = true;
                                update_ui = true;
                            }
                            else
                            {
                                ui_data.filters.push( val as u8 as char );
                                update_ui = true;
                            }
                        },
                        9 => // TAB key
                        {
                            if ui_data.nav_lock_refresh
                            {
                                ui_data.nav_lock_refresh = false;
                            }
                            else
                            {
                                ui_data.nav_lock_refresh = true;
                            }
                            update_ui = true;
                        },
                        _ => 
                        {
                            //ui_update(&format!("{}", val), &player_display, &mut ui_data, &encounters);
                            if ui_data.nav_lock_filter
                            {
                                ui_data.filters.push( val as u8 as char );
                                update_ui = true;
                            }
                        }
                        //ui_update(&format!("{}", val), &player_display, &mut ui_data, &encounters);}//ui_update(&format!("{}", encounters[0].attackers.len()), &player_display, &pointer, &encounters, &current_encounter);}//}
                    },
                Err(e) => {}
            }
            if update_ui && !encounters.is_empty()
            {
                ui_update(&format!(""), &player_display, &mut ui_data, &mut encounters);
                update_ui = false;
            }
        }
    });

    let mut attacks: Vec<structs::Attack> = Vec::new();
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
                        triggers.insert("Madness!", Regex::new(r".*Madness heals.*").unwrap());
                    for (trigger, trigged) in triggers.iter()
                    {
                        match trigged.captures(triggerbuffer.as_str()) {None => {}, Some(cap) =>
                        {
                            speak(&CString::new(format!("espeak \"{}\"", trigger)).unwrap());
                        }};
                    }
                });
                match re.captures(buffer.as_str()) {None => {/*println!("{}",buffer);*/}, Some(cap) =>
                {
                    fightdone = false;
                    attacks.push(structs::Attack::new());
                    attacks.last_mut().unwrap().attack(&cap, match cap.name("attacker").unwrap() { "" => player, var => var});
                    //encounter.encounter_end = parsed_time; //assume every line ends the encounter, likely not optimal, needs to be overhauled
                    battle_timer = time::Instant::now();
                }};
            }
            else /*Sleep for 0.1 sec if nothing has happened in the log-file*/
            {
                thread::sleep(time::Duration::from_millis(100));
            }
            /*update the UI, once every 1 sec*/
            if ui_update_timer.elapsed() >= time::Duration::from_millis(1000) && attacks.len() != 0 && !fightdone
            {
                ui_update_timer = time::Instant::now();
                parse_tx.send(Box::new((false, attacks.drain(0..).collect())));
            }
            /*End current encounter if nothing has been parsed in combat within the last 3 secs*/
            if battle_timer.elapsed() >= time::Duration::from_millis(3200)
            {
                if !fightdone
                {
                    attacks.clear();
                    fightdone = true;
                    parse_tx.send(Box::new((fightdone, attacks.drain(0..).collect())));
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
(1478706458)[Wed Nov  9 16:47:38 2016] YOUR Noxious Rending IV hits training dummy for 875382 disease damage.
(1478132824)[Thu Nov  3 01:27:04 2016] Devorstator's Impatience heals Devorstator for 43947 hit points.
YOU aoe attack training dummy for a critical of 204585 heat damage.

Enemy attacks are mis-parsed when they have a space in their name:
a deadwood harbinger gets parsed as name: "a" and attack_name: "deadwood harbinger"
this needs to be fixed in the regex when time allows, not that important at the moment though.

Attacks does not currently catch the "status", meaning if they double attack, flurry or AOE attack. This needs to be added.

attackers.print_attacks(filters: String) currently returns a printable string, this may need to change to a string vector to enable scrolling in the window and listing the line numbers.

println!("{}",buffer) <-- add this to the match X.captures() statement in the None body. Also needs to disable the code in the ui_update function.

*/
