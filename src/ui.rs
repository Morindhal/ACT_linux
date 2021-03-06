
use ncurses::*;
use json::JsonValue;
use std::time;


static ENCOUNTER_WINDOW_WIDTH: i32 = 30;

// TODO: Change all enums to have a usize value in order to hold the navigation instead of the current, more primitive, tuple-solution.
#[derive(PartialEq, Eq)]
pub enum PrimaryView
{
    EncounterList,
    CombatantList,
    CombatantInspect(usize),
    AbilityTrack(usize)
}

pub struct UiData
{
    pub nav_xy: Vec<(i32, i32, PrimaryView)>,
    pub nav_lock_encounter: bool,
    pub nav_lock_combatant: bool,
    pub nav_lock_filter: bool,
    pub nav_lock_refresh: bool,
    pub nav_main_win_scroll: (i32, i32),
    pub nav_encounter_win_scroll: (i32, i32),
    pub filters: String,
    pub debug: bool
}

impl UiData
{
    pub fn is_locked(&self) -> bool
    {
        if self.nav_lock_combatant || self.nav_lock_encounter || self.nav_lock_filter {true}
        else {false}
    }
    
    pub fn deeper(&mut self)
    {
        match self.nav_xy.last().unwrap().2
        {
            PrimaryView::EncounterList => self.nav_xy.push((0,0,PrimaryView::CombatantList)),
            PrimaryView::CombatantList => self.nav_xy.push((0,0,PrimaryView::CombatantInspect(0))),
            PrimaryView::CombatantInspect(_) => self.nav_xy.push((0,0,PrimaryView::AbilityTrack(0))),
            _ => {}
        }
    }
    
    pub fn surface(&mut self)
    {
        if self.nav_xy.len() > 0
            {self.nav_xy.pop();}
    }
    
    pub fn up(&mut self)
    {
        self.nav_xy.last_mut().unwrap().0 -=1;
    }
    
    pub fn down(&mut self)
    {
        self.nav_xy.last_mut().unwrap().0 +=1;
    }
    
    pub fn jsonify(&self)
        -> JsonValue
    {
        if !self.is_locked()
        {
            object!
            {
                "EncounterList" => true,
                "EncounterSpecific" => self.nav_xy.last().unwrap_or(&(0, 0, PrimaryView::EncounterList)).0
            }
        }
        else if self.nav_lock_combatant
        {
            object!
            {
                "EncounterList" => true,
                "EncounterSpecific" => self.nav_xy.last().unwrap_or(&(0, 0, PrimaryView::EncounterList)).0
            }
        }
        else if match self.nav_xy.last().unwrap().2 {PrimaryView::CombatantInspect(_) => true, _ => false}
        {
            object!
            {
                "EncounterList" => true,
                "EncounterSpecific" => self.nav_xy.last().unwrap_or(&(0, 0, PrimaryView::EncounterList)).0,
                "CombatantSpecific" => match self.nav_xy.last().unwrap().2 {PrimaryView::CombatantInspect(val) => val, _ => 0usize} //placeholder i32's, should be usize of the currently selected encounter/combatant
            }
        }
        //else if whatever view --> make json
        else
        {
            object!
            {
                "EncounterList" => true // if default, send EVERYTHING.
            }
        }
    }
}

pub fn ui_draw(highlight: &str, draw_object: &JsonValue, ui_data: &mut UiData)
{
    let mut max_x = 0;
    let mut max_y = 0;
    getmaxyx(stdscr(), &mut max_y, &mut max_x);

    ui_data.nav_main_win_scroll.0 = max_y - 22;
    ui_data.nav_encounter_win_scroll.0 = max_y - 22;

    

    let display_win = newwin(ui_data.nav_main_win_scroll.0, max_x-ENCOUNTER_WINDOW_WIDTH, 20,ENCOUNTER_WINDOW_WIDTH);
    let header_win = newwin(20, max_x, 0, 0);
    let encounter_list_win = newwin(ui_data.nav_encounter_win_scroll.0, ENCOUNTER_WINDOW_WIDTH, 20, 0);

    wclear(display_win);
    wclear(header_win);
    wclear(encounter_list_win);
    
    
    wmove(header_win, 1, 1);
    wprintw(header_win, " Welcome to ACT_linux!\n\n\n\tESC to exit.\n\tc to copy the last completed fight to the clipboard.\n\tC to copy the current fight to the clipboard.\n\tTAB to toggle a lock of the encounter-view to what is selected (X) or move to the newest encounter at each update.\n\t+ to begin editing the filters used to only  show certain attacks when inspecting a player.\n\n");
    wprintw(header_win, " Filters: ");
    wprintw(header_win, &ui_data.filters);
    

    if draw_object["EncounterSpecific"].is_null() != true
    {
        wmove(display_win, 1, 1);
        wattron(display_win, A_BOLD());
        wprintw(display_win, "\tEncounters:\n\n");
        wattroff(display_win, A_BOLD());
        //wprintw(display_win, draw_object.dump().as_str());
        
        for combatant in draw_object["EncounterSpecific"].members()
        {
            let duration = draw_object["EncounterList"][ui_data.nav_xy.last().unwrap().0 as usize]["Duration"].as_f64().unwrap_or(0f64);
            let dps = match duration{0.0=>0.0, _=>(combatant["Damage"].as_f64().unwrap_or(0f64) / duration)/1000000.0  };
            
            if combatant["Name"].as_str().unwrap().contains(highlight) {
                wattron(display_win, COLOR_PAIR(1));
                wprintw(display_win, &*build_string(combatant["Name"].as_str().unwrap(), dps));
                wattroff(display_win, COLOR_PAIR(1));
            }
            else {
                wprintw(display_win, &*build_string(combatant["Name"].as_str().unwrap(), dps));
            }
        }
    }
    else if draw_object["CombatantSpecific"].is_null() != true
    {
        wmove(display_win, 1, 1);
        wattron(display_win, A_BOLD());
        wprintw(display_win, "\tEncounters:\n\n");
        wattroff(display_win, A_BOLD());
        //wprintw(display_win, draw_object.dump().as_str());
        
        for attacks in draw_object["CombatantSpecific"].members() // should contain a list of what a ability did, parse that into %-ages.
        {
            wprintw(display_win, &*format!("SUPER ATTACK % LIST!!! {}", attacks));
        }
    }

    wmove(encounter_list_win, 1, 1);
    for encounter in draw_object["EncounterList"].members() {
        let duration = encounter["Duration"].as_u64().unwrap_or(0u64);
        let hours = (duration - duration%3600)/3600;
        let minutes = (duration - (duration-hours*3600)%60 ) / 60;
        let seconds = duration -hours*3600 -minutes*60;
        wprintw(encounter_list_win, &*format!(" {:02}:{:02}:{:02}\n", hours, minutes, seconds));
    }
    wmove(encounter_list_win, ui_data.nav_xy.last().unwrap().0+1, 1);
    

    wborder(display_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(header_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);
    wborder(encounter_list_win, '|' as chtype, '|' as chtype, '-' as chtype, '-' as chtype, '+' as chtype, '+' as chtype, '+' as chtype, '+' as chtype);

    wrefresh(display_win);
    wrefresh(header_win);
    wrefresh(encounter_list_win);

    delwin(display_win);
    delwin(header_win);
    delwin(encounter_list_win);
}

pub fn build_string(name: &str, dps: f64)
    -> String
{
    format!("     {name}: {dps:.3}m\n", name=name, dps=dps)
}

/*
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
                if ui_data.filters.len() != 0
                {
                    draw = format!("{} attacks:\n{}\n", 
                        encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name,
                        encounters[ui_data.nav_xy[0].0 as usize].print_attacks(&ui_data.filters, (&encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name)));
                }
                else
                {
                    draw = format!("{} attacks:\n{}\n", 
                        encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name,
                        encounters[ui_data.nav_xy[0].0 as usize].print_attack_stats(&encounters[ui_data.nav_xy[0].0 as usize].combatants[ui_data.nav_xy[1].0 as usize].name.as_str()));
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
*/
