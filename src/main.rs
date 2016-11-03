extern crate regex;
extern crate chrono;
extern crate libc;


use regex::{Regex};
use std::collections::HashMap;

use chrono::*;

use libc::system;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::io::SeekFrom;

use std::fmt;
use std::cmp::Ordering;

use std::{thread, time};


#[derive(Eq)]
struct Attack
{
    damage: u64,
    victim: String,
    timestamp: String,
    crit: String // "" for did not crit?
}

impl Attack
{
    fn attack(&mut self, attack_data: &regex::Captures)
    {
        
    }
}

impl Ord for Attack
{
    fn cmp(&self, other: &Attack) -> Ordering
    {
        self.damage.cmp(&other.damage)
    }
}

impl PartialOrd for Attack
{
    fn partial_cmp(&self, other: &Attack) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl PartialEq for Attack
{
    fn eq(&self, other: &Attack) -> bool
    {
        self.damage == other.damage
    }
}

#[derive(Eq)]
struct Attacker
{
    attacks: Vec<Attack>,
    final_damage: u64,
    final_healed: u64,
    name: String
}

impl Ord for Attacker
{
    fn cmp(&self, other: &Attacker) -> Ordering
    {
        other.final_damage.cmp(&self.final_damage)
    }
}

impl PartialOrd for Attacker
{
    fn partial_cmp(&self, other: &Attacker) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl PartialEq for Attacker
{
    fn eq(&self, other: &Attacker) -> bool
    {
        self.final_damage == other.final_damage
    }
}

impl Attacker
{
    fn attack(&mut self, attack_data: &regex::Captures)
    {
        self.attacks.push(Attack{damage: attack_data.name("damage").unwrap().parse::<u64>().unwrap(), victim: String::from(attack_data.name("target").unwrap()), timestamp: String::from(attack_data.name("datetime").unwrap()), crit: String::from(attack_data.name("crittype").unwrap())});
        self.final_damage += attack_data.name("damage").unwrap().parse::<u64>().unwrap();
    }
    
    fn print(&self, encounter_duration : u64) -> String
    {
        let dps = match encounter_duration{0=>0.0, _=>((self.final_damage / (encounter_duration)) as f64)/1000000.0  };
        let hps = match encounter_duration{0=>0.0, _=>((self.final_healed / (encounter_duration)) as f64)/1000000.0  };
        format!("{name:.*} \t  {dps:.2}m DPS\t{hps}k HPS\t", 4, name=self.name, dps=dps, hps=hps)
    }
}

struct Encounter
{
    attackers: Vec<Attacker>,
    encounter_start: DateTime<UTC>, //timestamp of when the fight started, get this from whatever starts the encounter
    encounter_end: DateTime<UTC>, //timestamp of when the fight ended, get this from whatever ends the encounter
    encounter_duration: u64 //duration of the encounter in nanoseconds, divide by 1000 to get seconds
}

impl Encounter
{
    fn exists(&self, name:&str) -> i32
    {
        for i in 0..((self.attackers).len())
        {
            if ((self.attackers))[i].name == name
            {
                return  i as i32;
            }
        }
        return -1;
    }
    
    fn attack(&mut self, attack_data: regex::Captures)
    {
        let attacker_name = match attack_data.name("attacker").unwrap() { "" => "Shepherd", var => var};
        if self.exists(attacker_name) == -1
        {
            (self.attackers).push(Attacker{attacks: Vec::new(), final_damage: 0, final_healed: 0, name: String::from(attacker_name)});
        }
        {
            let attackers_len = self.exists(attacker_name) as usize;
            self.attackers[attackers_len].attack(&attack_data);
        }
    }
    
    fn order(&mut self)
    {
        
    }
}


impl fmt::Display for Encounter
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {;
        let duration = (self.encounter_end-self.encounter_start);
        write!(f, "Encounter duration: {}:{}\n", duration.num_minutes(), duration.num_seconds() % 60 );
        for i in 0..((self.attackers).len())
        {
            write!(f, "{}\n", ((self.attackers))[i].print( duration.num_seconds() as u64 ));
        }
        write!(f, "\n")
    }
}

fn speak(data: &CStr) {
    extern { fn system(data: *const c_char); }

    unsafe { system(data.as_ptr()) }
}

fn main()
{
    let f = File::open("/media/bergman/Games/SteamLibrary/SteamApps/common/EverQuest 2/logs/Maj'Dul//eq2log_Shepherd.txt").unwrap();
    /*{
        Ok(file) => file,
        Err(e) => 
        {
            println!("{}", e);
        }
    };*/
    //Start a encounter, this code will be moved into the main loop when it works
    let mut encounters: Vec<Encounter> = Vec::new();
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>\D*?)(' |'s |YOUR |YOU )(?P<attack>\D*)(((multi attack)|hits|hit|flurry)|(( multi attacks)| hits| hit)) (?P<target>\D+) (?P<crittype>\D+) (?P<damage>\d+) (?P<damagetype>[A-Za-z]+) damage").unwrap();
    let timeparser = Regex::new(r"(?P<day_week>[A-Za-z]+) (?P<month>[A-Za-z]+)  (?P<day_month>\d+) (?P<hour>\d+):(?P<minute>\d+):(?P<second>\d+) (?P<year>\d+)").unwrap();

    let mut file = BufReader::new(&f);
    file.seek(SeekFrom::End(-10));
    
    let mut buffer = String::new();
    let mut battle_timer = time::Instant::now();
    let mut fightdone = true;
    'main: loop
    {
        buffer.clear();
        if file.read_line(&mut buffer).unwrap() > 0
        {
            let triggerbuffer = buffer.clone();
            thread::spawn( move || 
            {
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
                    //println!("{}",cap.name("attack").unwrap());
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
                    println!("\n\n\n\n\n");
                    encounters.push(Encounter{ attackers: Vec::new(), encounter_start: parsed_time, encounter_end: parsed_time, encounter_duration : 0});
                    fightdone = false;
                }
                encounters.last_mut().unwrap().attack(cap);
                encounters.last_mut().unwrap().encounter_duration = 0;// encounters.last_mut().unwrap().encounter_start.elapsed();
                encounters.last_mut().unwrap().encounter_end = parsed_time; //assume every line ends things
                battle_timer = time::Instant::now();
                }};
                }
            };
        }
        else
        {
            thread::sleep(time::Duration::from_millis(100));
        }
        if battle_timer.elapsed() >= time::Duration::from_millis(3000)
        {
            if !fightdone
            {
                encounters.last_mut().unwrap().attackers.sort();
                println!("{}", encounters.last().unwrap());
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

println!("{}",buffer

DATASTRUCTURE:
I should probably parse attacks into vectors as well, as there are a limited number of attacks viable and I am likely to want to be able to output all of a specific attack or the total damage of one particular attack
    This is advanced though so I won't do it for the first iteration of the program.

what I should be able to do:

* printout a full encounter as player dps, sorted by dps.
** This should be calculated by final damage divided by encounter_time in seconds

* printout a full encounter as player hps, sorted by hps
** This should be calculated by final healed divided by encounter_time in seconds

*/
