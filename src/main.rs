extern crate regex;


use regex::Regex;

use std::cell::Ref;
use std::cell::RefCell;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::io::SeekFrom;
use std::fmt;
use std::borrow::BorrowMut;
use std::any::Any;

use std::{thread, time};


struct Attack
{
    damage: u64,
    victim: String,
    timestamp: String,
    crit: String // "" for did not crit?
}

struct Attacker
{
    attacks: RefCell<Vec<Attack>>,
    final_damage: u64,
    final_healed: u64,
    name: String
}

impl Attacker
{
    fn attack(&self, attack_data: regex::Captures)
    {
        self.attacks.borrow_mut().push(Attack{damage: attack_data.name("damage").unwrap().parse::<u64>().unwrap(), victim: String::from(attack_data.name("target").unwrap()), timestamp: String::from(attack_data.name("datetime").unwrap()), crit: String::from(attack_data.name("crittype").unwrap())});
        println!("{}-{}-damage", self.name, self.final_damage);
    }
    
    fn print(&self) -> String
    {
        format!("Name : {}  -- DPS:  {}k\n", self.name, self.final_damage/1000)
    }
}

struct Encounter
{
    attackers: Vec<Attacker>,
    encounter_start: String, //timestamp of when the fight started, get this from whatever starts the encounter
    encounter_duration: u64 //duration of the encounter in nanoseconds, divide by 1000 to get seconds
}

impl Encounter
{
    fn exists(self, name:&str) -> bool
    {
        for i in 0..(self.attackers).len()
        {
            if ((self.attackers))[i].name == name
            {
                return  true;
            }
        }
        return false;
    }
    
    fn attack(self, attack_data: regex::Captures)
    {
        
        if !(self.exists(attack_data.name("attacker").unwrap()))
        {
            (self.attackers).push(Attacker{attacks: RefCell::new(Vec::new()), final_damage: 0, final_healed: 0, name: String::from(attack_data.name("attacker").unwrap())});
        }
        for i in 0..(self.attackers.len())
        {
            if self.attackers[i].name == attack_data.name("attacker").unwrap()
            {
                self.attackers[i].final_damage += attack_data.name("damage").unwrap().parse::<u64>().unwrap();
                (self.attackers)[i].attack(attack_data);
                break;
            }
        }
    }
}

impl fmt::Display for Encounter
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "Encounter:\n");
        for i in 0..((self.attackers).len())
        {
            write!(f, "{}", ((self.attackers))[i].print());
        }
        write!(f, "\nFight over")
    }
}

fn main()
{
    let f = File::open("/media/bergman/Games/SteamLibrary/SteamApps/common/EverQuest 2/logs/Maj'Dul/eq2log_Shepherd.txt").unwrap();
    /*{
        Ok(file) => file,
        Err(e) => 
        {
            println!("{}", e);
        }
    };*/

    //Start a encounter, this code will be moved into the main loop when it works
    let mut encounters: Vec<Encounter> = Vec::new();
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>[A-Za-z]+)( |'s)(?P<attack>\D+) ((multi attacks)|hits|hit) (?P<target>\D+) (?P<crittype>\D+) (?P<damage>\d+) (?P<damagetype>[A-Za-z]+) damage").unwrap();

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
            /*CHECK IF ATTACK
                CHECK ATTACKER, IF NO ATTACKER EXISTS CREATE A NEW ONE <-- this is done by String-matching the ATTACKERS name
                    PARSE ATTACK INTO ENCOUNTERS.ATTACKERS.ATTACK <-- this is done by String-matching the ATTACKERS name
            SAME FOR HEAL*/
            match re.captures(buffer.as_str()) {None => {}, Some(cap) =>
            {
           /* println!("{}", cap.name("attacker").unwrap());
            if cap.name("attacker").unwrap() == "YO"
            {
                println!("{}", buffer);
            }*/
               // println!("{}", battle_timer.elapsed().subsec_nanos());
                if fightdone
                {
                    println!("New fight!");
                    encounters.push(Encounter{ attackers: &mut *Vec::new(), encounter_start: String::from("START"), encounter_duration : 0});
                    fightdone = false;
                }
                encounters.last().unwrap().attack(cap);
                encounters.last_mut().unwrap().encounter_duration += battle_timer.elapsed().subsec_nanos() as u64;
                battle_timer = time::Instant::now();
            }};
        }
        else
        {
            thread::sleep(time::Duration::from_millis(100));
        }
        if battle_timer.elapsed() >= time::Duration::from_millis(3000)
        {
            if !fightdone
            {
                println!("{}", encounters.last().unwrap());
                fightdone = true;
            }
        }
    }
}

/*
(1477272172)[Mon Oct 24 03:22:52 2016] YOUR Raging Whirlwind multi attacks Bonesnapper for a critical of 38996 cold damage.\r\n
(1477272172)[Mon Oct 24 03:22:52 2016] Kabernet\'s Asylum multi attacks Bonesnapper for a critical of 36622 mental damage.\r\n
(1477646415)[Fri Oct 28 11:20:15 2016] YOUR Extreme Advantage hits training dummy for 11978400 crushing damage.



DATASTRUCTURE:
I should probably parse attacks into vectors as well, as there are a limited number of attacks viable and I am likely to want to be able to output all of a specific attack or the total damage of one particular attack
    This is advanced though so I won't do it for the first iteration of the program.

what I should be able to do:

* printout a full encounter as player dps, sorted by dps.
** This should be calculated by final damage divided by encounter_time in seconds

* printout a full encounter as player hps, sorted by hps
** This should be calculated by final healed divided by encounter_time in seconds

*/
