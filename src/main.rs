extern crate libc;
extern crate clap;
extern crate clipboard;
extern crate ncurses;
extern crate mmo_parser_backend;
#[macro_use]
extern crate json;

use std::sync::mpsc::{self, RecvTimeoutError};

use libc::system;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

use clap::{Arg, App};

use clipboard::ClipboardContext;

use ncurses::*;

use std::fmt;

use std::{thread, time};


mod ui;

use mmo_parser_backend::eventloop::EventLoop;


fn speak(data: &CStr) {
    extern { fn system(data: *const c_char); }

    unsafe { system(data.as_ptr()) }
}



fn main()
{
    let matches = App::new("mmo_parser_cli")
        .version("0.1.0")
        .author("Bergman. <Morindhal@gmail.com>")
        .about("Parses MMO logs")
            .arg(Arg::with_name("FILE")
                .help("Sets the log-file to use")
                .required(true)
                .index(1))
            .arg(Arg::with_name("player")
                .required(true)
                .help("Sets the character name to parse, this only catches the YOU and YOUR lines"))
            .arg(Arg::with_name("GAME"))
                .help("Sets the game to parse, EQ2 is default")
        .get_matches();
    /*Set log-file and player whos view the combat is parsed from based on CL input, player should be replaced with a name collected from the file-string*/
    let from_file = matches.value_of("FILE").unwrap();
    let player = matches.value_of("player").unwrap();
    let player_display = String::from(player);


    let (send_data_request, recieve_answer) = EventLoop::new(String::from(from_file), String::from(player));

    /*start the n-curses UI*/
    initscr();
    keypad(stdscr(), true);
    noecho();
    start_color();
    init_pair(1, COLOR_RED, COLOR_BLACK);


    let (input_send, input_recieve) = mpsc::channel();
    //let (timer_tx, timer_rx) = mpsc::channel();

    
    
    let buttonlistener = thread::spawn(move || 
    {
        'input: loop/*Listen to input, send input to main*/
        {
            input_send.send(getch()).unwrap();
        }
    });
    
/*    let timer_counter = thread::spawn(move || 
    {
        let timeout = time::Duration::from_millis(500);
        let mut time = time::Instant::now();
        let mut timers: HashMap<&str, i32> = HashMap::new();
        'timer: loop//Listen to timers, give tts warning when time is up
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

    /*
    * Send request for data
    * recieve data, wait for data until data is sent.
    */
    let mut jsonobject:json::JsonValue = object!{"initial" => true};
    send_data_request.send(Box::new(object!{"initial" => true}));
    jsonobject = *(recieve_answer.recv().unwrap());
    

    let mut ctx = ClipboardContext::new().unwrap();
    let timeout = time::Duration::from_millis(1);
    let mut ui_data = ui::ui_data{nav_xy: vec![(0,0)], nav_lock_encounter: false, nav_lock_combatant: false, nav_lock_filter: false, nav_lock_refresh: true, nav_main_win_scroll: (0, 0), nav_encounter_win_scroll: (5, 0), filters: String::from(""), debug: false};
    let mut update_ui = true;
    
    let mut update_tick = time::Instant::now();
    
    'ui: loop
    {
        if update_tick.elapsed() >= time::Duration::from_millis(1000)
        {
            update_tick = time::Instant::now();
            send_data_request.send(Box::new(ui_data.jsonify()));
            match recieve_answer.recv()
            {
                Ok(val) =>
                {
                    jsonobject = *val;
                },
                Err(e) => {}
            }
        }
        if !ui_data.nav_lock_encounter && ui_data.nav_lock_refresh
        {
            if jsonobject["attacks"].len() != 0 && !ui_data.is_locked()
            {
                ui_data.nav_xy[0].0 = jsonobject["encounters"].len() as i32 - 1;
                ui_data.nav_encounter_win_scroll.1 = 0;
            }
        }
        match input_recieve.recv_timeout(timeout)
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
                            match ctx.set_contents(format!("{}", jsonobject["encounters"][ui_data.nav_xy[0].0 as usize - ui_data.nav_encounter_win_scroll.1 as usize]))//if ui_data.nav_xy[0].0 >= encounters.len() as i32 {&encounters[encounters.len()-1 as usize]} else {&encounters[ui_data.nav_xy[0].0 as usize]}))
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
                                if  ui_data.nav_encounter_win_scroll.0 - 2 < jsonobject["encounters"].len() as i32
                                {
                                    if ui_data.nav_encounter_win_scroll.1 < jsonobject["encounters"].len() as i32 - ui_data.nav_encounter_win_scroll.0 - 2
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
                            else if ui_data.nav_lock_encounter && jsonobject["combatants"].len() as i32 - ui_data.nav_main_win_scroll.0 - ui_data.nav_main_win_scroll.1 < 0 ||
                                    ui_data.nav_lock_encounter && jsonobject["combatants"].len() as i32 - ui_data.nav_main_win_scroll.0 - ui_data.nav_main_win_scroll.1 < 0
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
                        if ui_data.nav_lock_encounter && ui_data.nav_xy[1].0 < jsonobject["combatants"].len() as i32 - 1
                        {
                            ui_data.nav_xy.last_mut().unwrap().0 += 1;
                        }
                        else if ui_data.nav_lock_combatant
                        {
                            ui_data.nav_xy.last_mut().unwrap().0 += 1;
                        }
                        else if !ui_data.is_locked() && ui_data.nav_xy.last().unwrap().0 < jsonobject["encounters"].len() as i32 - 1
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
                            if ui_data.nav_xy.last().unwrap().0 == jsonobject["encounters"].len() as i32
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
        /*
        * parse what data the program is currently interested in and send that in a request to the Sender recieved from mmo_parser_backend
        * parse the response and update the display.
        * 
        * do this procedure ONLY if a request for new data has been sent.
        * 
        * poll for update every X secs.
        * OR
        * listen to updates from the parser.
        */
        if update_ui
        {
            ui::ui_draw(&format!(""), &player_display, &jsonobject, &mut ui_data);
            update_ui = false;
        }
        thread::sleep(time::Duration::from_millis(100));
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


println!("{}",buffer) <-- add this to the match X.captures() statement in the None body. Also needs to disable the code in the ui_update function.

send a request-JSON-object to the parser, this contains a wish-list of what I want.
    The response is then forwarded to the update_ui function and drawn.
        This has the benefit of being doable even if the response doesn't exactly match expectations.
    This request changes depending on user input.
        Limit how many resonses that are wanted by low+count?


*/
