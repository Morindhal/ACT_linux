extern crate libc;
extern crate clap;
extern crate clipboard;
extern crate ncurses;
extern crate mmo_parser_backend;
#[macro_use]
extern crate json;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::sync::mpsc::{self};

use std::ffi::{CString, CStr};
use std::os::raw::c_char;

use clap::{Arg, App};

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

use ncurses::*;

use std::{thread, time};


mod ui;

use mmo_parser_backend::eventloop::event_loop;
use log::LogLevel;


fn speak(data: &CStr) {
    extern { fn system(data: *const c_char); }

    unsafe { system(data.as_ptr()) }
}



fn main()
{
    env_logger::init().unwrap();
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
    let from_file = match matches.value_of("FILE") {Some(f) => f , None => {warn!("Unable to parse FILE into a variable."); panic!("Unable to parse FILE into a variable");}};
    let player = match matches.value_of("player") {Some(p) => p , None => {warn!("Unable to parse FILE into a variable."); "NONAME"}};
    let player_display = String::from(player);


    let (send_data_request, recieve_answer) = event_loop::new(String::from(from_file), String::from(player));

    /*start the n-curses UI*/
    initscr();
    keypad(stdscr(), true);
    noecho();
    start_color();
    init_pair(1, COLOR_RED, COLOR_BLACK);


    let (input_send, input_recieve) = mpsc::channel();
    //let (timer_tx, timer_rx) = mpsc::channel();

    
    
    thread::spawn(move || 
    {
        'input: loop/*Listen to input, send input to main*/
        {
            match input_send.send(getch()) {Ok(_) => {}, Err(e) => {warn!("Error sending input to the UI-sync thread :  {}", e);}};
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
    let mut jsonobject:json::JsonValue;
    match send_data_request.send(Box::new(object!{"initial" => true})) {Ok(_) => {}, Err(e) => {warn!("Error sending input to the parser backend :  {}", e);}};
    jsonobject = *(recieve_answer.recv().unwrap());

    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let timeout = time::Duration::from_millis(1);
    let mut ui_data = ui::UiData{nav_xy: vec![(0,0,ui::PrimaryView::EncounterList)], nav_lock_encounter: false, nav_lock_combatant: false, nav_lock_filter: false, nav_lock_refresh: true, nav_main_win_scroll: (0, 0), nav_encounter_win_scroll: (5, 0), filters: String::from(""), debug: false};
    let mut update_ui = true;
    
    let mut update_tick = time::Instant::now();
    
    'ui: loop
    {
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
                            match ctx.set_contents(format!("{}",
                                { 
                                    let duration = jsonobject["EncounterList"][ui_data.nav_xy.last().unwrap().0 as usize]["Duration"].as_f64().unwrap_or(0f64);
                                    let mut returnstring = format!("Duration: {}:{}\n", (duration/60f64) as usize, duration%60f64);
                                    for combatant in jsonobject["EncounterSpecific"].members()
                                    {
                                        let dps = match duration{0.0=>0.0, _=>(combatant["Damage"].as_f64().unwrap_or(0f64) / duration)/1000000.0  };
                                        returnstring += format!("{name:.4}: {dps:.0}m\n", name=combatant["Name"].as_str().unwrap(), dps=dps).as_str();
                                    }returnstring}))
                                {
                                    Ok(_)=>
                                    {
                                        /*This is currently linux dependant, probably not the best idea for future alerts but for now it "works" assuming one has the correct file on the system*/
                                        speak(&CString::new(format!("paplay /usr/share/sounds/freedesktop/stereo/message.oga")).unwrap());
                                    },
                                    Err(e)=>{warn!("ERROR with the clipboard : {}", e);}
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
                        if ui_data.nav_xy.last_mut().unwrap().0 > 0 {
                            ui_data.up();
                            update_ui = true;
                        }
                    },
                    KEY_DOWN => 
                    {
                        if ui_data.nav_lock_encounter && ui_data.nav_xy[1].0 < jsonobject["EncounterSpecific"].len() as i32 - 1
                        {
                            ui_data.nav_xy.last_mut().unwrap().0 += 1;
                        }
                        else if ui_data.nav_lock_combatant
                        {
                            ui_data.nav_xy.last_mut().unwrap().0 += 1;
                        }
                        else if !ui_data.is_locked() && ui_data.nav_xy.last().unwrap().0 < jsonobject["EncounterList"].len() as i32 - 1
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
                            if ui_data.nav_xy.last().unwrap().0 == jsonobject["EncounterList"].len() as i32
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
            Err(e) => {warn!("ERROR recieving data from user input : {}", e);}
        }
        if update_ui || update_tick.elapsed() >= time::Duration::from_millis(3000)
        {
            update_tick = time::Instant::now();
            send_data_request.send(Box::new(ui_data.jsonify())).unwrap();// {Ok(_) => {}, Err(e) => {warn!("Error sending input to the parser backend :  {}", e);}};
            match recieve_answer.recv()
            {
                Ok(val) =>
                {
                    jsonobject = *val;
                    if ui_data.nav_xy.last().unwrap().2 == ui::PrimaryView::EncounterList && !update_ui// && !ui_data.nav_lock_refresh
                    {
                        ui_data.nav_xy[0].0 = jsonobject["EncounterList"].len() as i32 - 1;
                        ui_data.nav_encounter_win_scroll.1 = 0;
                    }
                    update_ui = true;
                },
                Err(e) => {}//warn!("ERROR recieving answer from the parser backend : {}", e);}
            }
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
            ui::ui_draw(&player_display, &jsonobject, &mut ui_data);
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
        
TODO: Retool navigation.


*/
