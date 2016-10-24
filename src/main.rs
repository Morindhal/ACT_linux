extern crate regex;


use regex::Regex;

use std::cell::Ref;
use std::cell::RefCell;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;

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

struct Encounter
{
    attackers: RefCell<Vec<Attacker>>,
    encounter_start: String, //timestamp of when the fight started, get this from whatever starts the encounter
    encounter_duration: u64 //duration of the encounter in nanoseconds, divide by 1000 to get seconds
}

impl Encounter
{
    fn exists(&self, name:&str) -> bool
    {
        for i in 0..((self.attackers.borrow()).len())
        {
            if ((self.attackers).borrow())[i].name == name
            {
                return  true;
            }
        }
        return false;
    }
    
    fn add_attacker(&self, attacker: &str)
    {
        println!("Totes adding it, just don't check!");
        (self.attackers).borrow_mut().push(Attacker{attacks: RefCell::new(Vec::new()), final_damage: 0, final_healed: 0, name: String::from(attacker)});
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
    encounters.push(Encounter{ attackers: RefCell::new(Vec::new()), encounter_start: String::from("START"), encounter_duration : 0});
    
    let re = Regex::new(r"\((?P<time>\d+)\)\[(?P<datetime>(\D|\d)+)\] (?P<attacker>\D+)(YOUR|'s) (?P<attack>\D+) ((multi attacks)|multi) (?P<target>\D+) for a (?P<crittype>\D+) of (?P<damage>\d+) (cold|heat|mental|arcane|poison|noxious) damage").unwrap();

    let mut file = BufReader::new(&f);
    
    let mut buffer = String::new();
    let mut battle_timer = time::Instant::now();
    'main: loop
    {
        if file.read_line(&mut buffer).unwrap() > 0
        {
            /*CHECK IF ATTACK
                CHECK ATTACKER, IF NO ATTACKER EXISTS CREATE A NEW ONE <-- this is done by String-matching the ATTACKERS name
                    PARSE ATTACK INTO ENCOUNTERS.ATTACKERS.ATTACK <-- this is done by String-matching the ATTACKERS name
            SAME FOR HEAL*/
            let temp = re.captures(buffer.as_str()) ;
            match temp {None => {}, Some(cap) =>
            {
                //look to see if the attacker already has a post, if so place the attack there, if not push a new attacker
                if encounters[0].exists(cap.name("attacker").unwrap())
                {
                    println!("He's there alrighty!");
                }
                else
                {
                    encounters[0].add_attacker(cap.name("attacker").unwrap());
                }
                //encounters[0].attackers.cap.name("datetime").unwrap()
            }};
            //if buffer.find("attacks") != ;
            encounters[0].encounter_duration += battle_timer.elapsed().subsec_nanos() as u64;
            battle_timer = time::Instant::now();
        }
        else
        {
            thread::sleep(time::Duration::from_millis(100));
            if battle_timer.elapsed() >= time::Duration::from_millis(3000)
            {
                println!("New battle incoming, {:?} time has elapsed since the last one.", battle_timer.elapsed().as_secs());
            }
        }
        buffer.clear();
    }
}

/* \[P<DATE>\]
\((?P<time>\d+)\)
(1477272172)[Mon Oct 24 03:22:52 2016] YOUR Raging Whirlwind multi attacks Bonesnapper for a critical of 38996 cold damage.\r\n
(1477272172)[Mon Oct 24 03:22:52 2016] Kabernet\'s Asylum multi attacks Bonesnapper for a critical of 36622 mental damage.\r\n



DATASTRUCTURE:
I should probably parse attacks into vectors as well, as there are a limited number of attacks viable and I am likely to want to be able to output all of a specific attack or the total damage of one particular attack
    This is advanced though so I won't do it for the first iteration of the program.

what I should be able to do:

* printout a full encounter as player dps, sorted by dps.
** This should be calculated by final damage divided by encounter_time in seconds

* printout a full encounter as player hps, sorted by hps
** This should be calculated by final healed divided by encounter_time in seconds

*/
