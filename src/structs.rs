extern crate regex;
use regex::{Regex};
use std::cmp::Ordering;
use std::fmt;
use std::{thread, time};
use chrono::*;


pub struct ui_data
{
    pub nav_xy: Vec<(i32, i32)>,
    pub nav_lock_encounter: bool,
    pub nav_lock_combatant: bool,
    pub nav_lock_filter: bool,
    pub nav_lock_refresh: bool,
    pub nav_main_win_scroll: (i32, i32),
    pub nav_encounter_win_scroll: (i32, i32),
    pub filters: String,
    pub debug: bool
}

impl ui_data
{
    pub fn is_locked(&self) -> bool
    {
        if self.nav_lock_combatant || self.nav_lock_encounter || self.nav_lock_filter {true}
        else {false}
    }

    pub fn unlock(&mut self)
    {
        self.nav_lock_combatant = false;
        self.nav_lock_encounter = false;
        self.nav_lock_filter = false;
    }
    
    pub fn deeper(&mut self)
    {
        self.nav_xy.push((0,0));
    }
    
    pub fn surface(&mut self)
    {
        self.nav_xy.pop();
    }
}

#[derive(Eq, Clone)]
pub struct Attack
{
    attacker: String,
    damage: u64,
    victim: String,
    pub timestamp: String,
    pub attack_name: String,
    crit: String, // "" for did not crit?
    damage_type: String
}

impl Attack
{
    pub fn attack(&mut self, attack_data: &regex::Captures, attacker: &str)
    {
        self.attacker = String::from(attacker);
        self.damage = attack_data.name("damage").unwrap().as_str().parse::<u64>().unwrap();
        self.victim = String::from(attack_data.name("target").unwrap().as_str());
        self.timestamp = String::from(attack_data.name("datetime").unwrap().as_str());
        self.attack_name = String::from(match attack_data.name("attack").unwrap().as_str() { "" => "auto attack", val => val } );
        self.crit = String::from(attack_data.name("crittype").unwrap().as_str());
        self.damage_type = String::from(attack_data.name("damagetype").unwrap().as_str());
    }
    
    pub fn filter(&self, filters: &str, attacker: &String) -> bool
    {
        if !self.attacker.contains(attacker) {return false;}
        if filters.len() as i32 != 0
        {
            for filter in filters.split_whitespace()
            {
                if !self.timestamp.contains(filter) && !self.victim.contains(filter) && !self.attack_name.contains(filter) && !self.crit.contains(filter) && !self.damage_type.contains(filter)  {return false;}
            }
        }
        true
    }
    
    pub fn new()
        -> Attack
    {
        Attack
        {
            attacker: String::from("undefined"),
            damage: 0,
            victim: String::from("undefined"),
            timestamp: String::from("undefined"),
            attack_name: String::from("undefined"),
            crit: String::from("undefined"),
            damage_type: String::from("undefined")
        }
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

impl fmt::Display for Attack
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{:25.25}   VICTIM: {:20.20}   ATTACK: {:30.30}   DAMAGE: {:>15.15}   CRIT: {:>20.20}   TYPE: {:>10.10}", self.timestamp, self.victim, self.attack_name, self.damage, self.crit, self.damage_type)
        //write!(f, "{}", self.timestamp, self.victim, self.attack_name)
    }
}


#[derive(Eq)]
pub struct Attack_Stats
{
    name: String,
    attackNmbr: usize,
    totalDamage: u64
}

impl Attack_Stats
{
    pub fn find_attackname(&mut self, attacks: &Vec<Attack>, attackNmbr: usize)
        -> bool
    {
        if self.name == attacks[attackNmbr].attack_name
        {
            if attacks[self.attackNmbr].damage < attacks[attackNmbr].damage
            {self.attackNmbr = attackNmbr;}
            self.totalDamage += attacks[attackNmbr].damage;
            true
        }
        else
        {false}
    }
    
    pub fn print(&self, duration: u64, allDamage: u64, attacks: &Vec<Attack>)
        -> String
    {
        format!("{:6.2} procent of parse   {}\n", (self.totalDamage as f64 / allDamage as f64 * 100.0), (attacks[self.attackNmbr]))
    }

    pub fn new(attacks: &Vec<Attack>, attackNmbr: usize)
        -> Attack_Stats
    {
        Attack_Stats{name: attacks[attackNmbr].attack_name.clone(), attackNmbr: attackNmbr, totalDamage: attacks[attackNmbr].damage}
    }

}

impl Ord for Attack_Stats
{
    fn cmp(&self, other: &Attack_Stats) -> Ordering
    {
        other.totalDamage.cmp(&self.totalDamage)
    }
}

impl PartialOrd for Attack_Stats
{
    fn partial_cmp(&self, other: &Attack_Stats) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl PartialEq for Attack_Stats
{
    fn eq(&self, other: &Attack_Stats) -> bool
    {
        other.totalDamage == self.totalDamage
    }
}



#[derive(Eq)]
pub struct Attacker
{
    attacks: Vec<Attack>,
    final_damage: u64,
    final_healed: u64,
    pub name: String
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
    pub fn attack(&mut self, attack_data: &regex::Captures)
    {
        //self.attacks.push(Attack{damage: attack_data.name("damage").unwrap().parse::<u64>().unwrap(), victim: String::from(attack_data.name("target").unwrap()), timestamp: String::from(attack_data.name("datetime").unwrap()), attack_name: String::from(match attack_data.name("attack").unwrap() { "" => "auto attack", val => val } ), crit: String::from(attack_data.name("crittype").unwrap()), damage_type: String::from(attack_data.name("damagetype").unwrap())});
        self.final_damage += attack_data.name("damage").unwrap().as_str().parse::<u64>().unwrap();
    }
    
    /*This should probably be replaced by a impl fmt::Display*/
    pub fn print(&self, encounter_duration : u64) -> String
    {
        let dps = match encounter_duration{0=>0.0, _=>((self.final_damage / (encounter_duration)) as f64)/1000000.0  };
        /*Leave this commented until heals are parsed*/
        //let hps = match encounter_duration{0=>0.0, _=>((self.final_healed / (encounter_duration)) as f64)/1000.0  };
        //format!("{name:.*}: {dps:.1}m | {hps}k", 4, name=self.name, dps=dps, hps=hps)
        format!("{name:.*}: {dps:.1}m ", 4, name=self.name, dps=dps)
    }

    /*This should probably be replaced by a impl fmt::Debug*/
    pub fn print_full(&self, encounter_duration : u64) -> String
    {
        let dps = match encounter_duration{0=>0.0, _=>((self.final_damage / (encounter_duration)) as f64)/1000000.0  };
        /*Leave this commented until heals are parsed*/
        //let hps = match encounter_duration{0=>0.0, _=>((self.final_healed / (encounter_duration)) as f64)/1000.0  };
        //format!("{name:.*}: {dps:.1}m | {hps}k", 4, name=self.name, dps=dps, hps=hps)
        format!("{name}: {dps:.3}m ", name=self.name, dps=dps)
    }
}

impl Clone for Attacker
{
    fn clone(&self) -> Attacker{ Attacker{attacks: self.attacks.clone(), final_damage: self.final_damage, final_healed: self.final_healed, name: self.name.clone()} }
}

impl fmt::Display for Attacker
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        for i in 0..((self.attacks).len())
        {
            write!(f, "{}\n", ((self.attacks))[i]);
        }
        write!(f, "")
    }
}


pub struct Encounter
{
    pub attackers: Vec<Attacker>,
    pub encounter_start: DateTime<UTC>, //timestamp of when the fight started, get this from whatever starts the encounter
    pub encounter_end: DateTime<UTC>, //timestamp of when the fight ended, get this from whatever ends the encounter
    pub encounter_duration: u64, //duration of the encounter in nanoseconds, divide by 1000 to get seconds
    pub player: String
}

impl Encounter
{
    pub fn exists(&self, name:&str) -> i32
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
    
    pub fn attack(&mut self, attack_data: regex::Captures)
    {
        let attacker_name = match attack_data.name("attacker").unwrap().as_str() { "" => self.player.as_str(), var => var};
        if self.exists(attacker_name) == -1
        {
            (self.attackers).push(Attacker{attacks: Vec::new(), final_damage: 0, final_healed: 0, name: String::from(attacker_name)});
        }
        {
            let attackers_len = self.exists(attacker_name) as usize;
            self.attackers[attackers_len].attack(&attack_data);
        }
    }
    
    pub fn order(&mut self)
    {
        
    }
}

impl fmt::Debug for Encounter
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let duration = (self.encounter_end-self.encounter_start);
        write!(f, "Encounter duration: {}:{:02}\n", duration.num_minutes(), duration.num_seconds() % 60 );
        for i in 0..((self.attackers).len())
        {
            write!(f, "{}\n", ((self.attackers))[i].print_full( duration.num_seconds() as u64 ));
        }
        write!(f, "")
    }
}

impl fmt::Display for Encounter
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let duration = (self.encounter_end-self.encounter_start);
        write!(f, "Encounter duration: {}:{:02}\n", duration.num_minutes(), duration.num_seconds() % 60 );
        for i in 0..((self.attackers).len())
        {
            write!(f, "{}\n", ((self.attackers))[i].print( duration.num_seconds() as u64 ));
        }
        write!(f, "")
    }
}

impl Clone for Encounter
{
    fn clone(&self) -> Encounter{ Encounter{attackers: self.attackers.clone(), encounter_start: self.encounter_start.clone(), encounter_end: self.encounter_end.clone(), encounter_duration: self.encounter_duration, player: self.player.clone()} }
}


pub struct CombatantList
{
    pub combatants: Vec<Combatant>,
    pub attacks: Vec<Attack>,
    pub attack_stats: Vec<Attack_Stats>,
    pub encounter_start: DateTime<UTC>,
    pub encounter_end: DateTime<UTC>,
    pub encounter_duration: u64,
    pub highestHit: Attack,
    pub highestHeal: Attack
}

impl CombatantList
{
    pub fn attack(&mut self, mut attack: Attack)
    {
        if self.attacks.len() == 0
        {self.encounter_start = getTime(attack.timestamp.as_str());}
        self.encounter_end = getTime(attack.timestamp.as_str());

        
        match self.find_combatant(attack.attacker.as_str())
        {
            -1 =>/*New attacker*/
                {
                    self.combatants.push(Combatant{name: attack.attacker.clone(), highestHit: Attack::new(), highestHeal: Attack::new(), final_healed: 0, final_damage: 0, attack_stats: Vec::new(), combatstart: getTime(attack.timestamp.as_str()), sortByDps: true});
                    self.attacks.push(attack);
                    self.combatants.last_mut().unwrap().attack(&self.attacks, self.attacks.len()-1);
                    self.combatants.last_mut().unwrap().final_damage += self.attacks.last().unwrap().damage;
                },
            i =>
            {
                self.combatants[i as usize].final_damage += attack.damage;
                self.attacks.push(attack);
                self.combatants[i as usize].attack(&self.attacks, self.attacks.len()-1);
            },
        };
        /*enter the attack data into a list that keeps track of specific attacks
        * This list MUST also be entered on a player-level, create one list-struct for both?
        */
        {
            let mut exists = false;
            for stats in self.attack_stats.iter_mut()
            {
                exists = stats.find_attackname(&self.attacks, self.attacks.len()-1);
                if exists {break;}
            }
            if !exists
            {self.attack_stats.push(Attack_Stats::new(&self.attacks, self.attacks.len()-1));}
        }
    }
    
    pub fn find_combatant(&mut self, attacker: &str)
        -> i32
    {
        for i in 0..self.combatants.len()
        {
            if self.combatants[i].name == attacker
            {return i as i32;}
        }
        -1
    }
    
    pub fn new(start: DateTime<UTC>)
        -> CombatantList
    {
        CombatantList{combatants: Vec::new(), attacks: Vec::new(), attack_stats: Vec::new(), encounter_start: start, encounter_end: start, encounter_duration: 0, highestHit: Attack::new(), highestHeal: Attack::new()}
    }
    
    pub fn print_attacks(&self, filters: &str, player: &String) -> String
    {
        let mut results: String = String::from("");
        for attack in &self.attacks
        {
            if attack.filter(filters, &player)
            {
                results.push_str(&format!("{}\n", attack));
            }
        }
        results
    }

    pub fn print_attack_stats(&self, player: &str) -> String
    {
        let mut results: String = String::from("");
        for combatant in &self.combatants
        {
            if combatant.name == player
            {
                for stats in &combatant.attack_stats
                {
                    results.push_str(&format!("{}", stats.print((self.encounter_end-self.encounter_start).num_seconds() as u64, combatant.final_damage, &self.attacks)));
                }
            }
        }
        results
    }
}

impl fmt::Display for CombatantList
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let duration = (self.encounter_end-self.encounter_start);
        write!(f, "Encounter duration: {}:{:02}\n", duration.num_minutes(), duration.num_seconds() % 60 );
        for i in 0..((self.combatants).len())
        {
            write!(f, "{}\n", ((self.combatants))[i].print( duration.num_seconds() as u64 ));
        }
        write!(f, "")
    }
}

impl fmt::Debug for CombatantList
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let duration = (self.encounter_end-self.encounter_start);
        write!(f, "Encounter duration: {}:{:02}\n", duration.num_minutes(), duration.num_seconds() % 60 );
        for i in 0..((self.combatants).len())
        {
            write!(f, "{}\n", ((self.combatants))[i].print_full( duration.num_seconds() as u64 ));
        }
        write!(f, "")
    }
}

#[derive(Eq)]
pub struct Combatant
{
    pub name: String,
    pub highestHit: Attack,
    pub highestHeal: Attack,
    pub final_healed: u64,
    pub final_damage: u64,
    pub attack_stats: Vec<Attack_Stats>,
    pub combatstart: DateTime<UTC>,
    pub sortByDps: bool
}

impl Ord for Combatant
{
    fn cmp(&self, other: &Combatant) -> Ordering
    {
        if self.sortByDps
        {
            other.final_damage.cmp(&self.final_damage)
        }
        else
        {
            other.final_healed.cmp(&self.final_healed)
        }
    }
}

impl PartialOrd for Combatant
{
    fn partial_cmp(&self, other: &Combatant) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl PartialEq for Combatant
{
    fn eq(&self, other: &Combatant) -> bool
    {
        if self.sortByDps
        {
            other.final_damage == self.final_damage
        }
        else
        {
            other.final_healed == self.final_healed
        }
    }
}

impl Combatant
{
    pub fn print(&self, encounter_duration : u64) -> String
    {
        let dps = match encounter_duration{0=>0.0, _=>((self.final_damage / (encounter_duration)) as f64)/1000000.0  };
        /*Leave this commented until heals are parsed*/
        //let hps = match encounter_duration{0=>0.0, _=>((self.final_healed / (encounter_duration)) as f64)/1000.0  };
        //format!("{name:.*}: {dps:.1}m | {hps}k", 4, name=self.name, dps=dps, hps=hps)
        format!("{name:.*}: {dps:.1}m ", 4, name=self.name, dps=dps)
    }

    /*This should probably be replaced by a impl fmt::Debug*/
    pub fn print_full(&self, encounter_duration : u64) -> String
    {
        let dps = match encounter_duration{0=>0.0, _=>((self.final_damage / (encounter_duration)) as f64)/1000000.0  };
        /*Leave this commented until heals are parsed*/
        //let hps = match encounter_duration{0=>0.0, _=>((self.final_healed / (encounter_duration)) as f64)/1000.0  };
        //format!("{name:.*}: {dps:.1}m | {hps}k", 4, name=self.name, dps=dps, hps=hps)
        format!("{name}: {dps:.3}m ", name=self.name, dps=dps)
    }

/*    pub fn print_attack_stats(&self, encounter_duration : u64) -> String
    {
        let mut results: String = String::from("");
        for stats in &self.attack_stats
        {
            results.push_str(&format!("{}", stats.print(encounter_duration));
        }
        results
    }*/
    
    pub fn attack(&mut self, mut attacks: &Vec<Attack>, attackNmbr: usize)
    {
        let mut exists = false;
        for stats in self.attack_stats.iter_mut()
        {
            exists = stats.find_attackname(&attacks, attackNmbr);
            if exists {break;}
        }
        if !exists
        {self.attack_stats.push(Attack_Stats::new(&attacks, attackNmbr));}
        self.attack_stats.sort();
    }
}

pub fn getTime(timestamp: &str)
    -> DateTime<UTC>
{
    let timeparser = Regex::new(r"(?P<day_week>[A-Za-z]+) (?P<month>[A-Za-z]+)(  | )(?P<day_month>\d+) (?P<hour>\d+):(?P<minute>\d+):(?P<second>\d+) (?P<year>\d+)").unwrap();
    match timeparser.captures( timestamp ) {None => {return UTC.ymd(2016, 2, 3).and_hms(0, 0, 0);}, Some(time_cap) =>
    {
        return UTC
                                .ymd(
                                    time_cap.name("year").unwrap().as_str().parse::<i32>().unwrap(),
                                    match time_cap.name("month").unwrap().as_str() {"Jan"=>1, "Feb"=>2, "Mar"=>3, "Apr"=>4,  "May"=>5, "Jun"=>6, "Jul"=>7, "Aug"=>8, "Sep"=>9, "Oct"=>10, "Nov"=>11, "Dec"=>12, _=>1},
                                    time_cap.name("day_month").unwrap().as_str().parse::<u32>().unwrap())
                                .and_hms(
                                    time_cap.name("hour").unwrap().as_str().parse::<u32>().unwrap(),
                                    time_cap.name("minute").unwrap().as_str().parse::<u32>().unwrap(),
                                    time_cap.name("second").unwrap().as_str().parse::<u32>().unwrap()
                                    );
    }};
}
