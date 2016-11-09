extern crate regex;
use regex::{Regex};
use std::cmp::Ordering;
use std::fmt;
use std::{thread, time};
use chrono::*;


pub struct ui_data
{
    pub pointer: (i32,i32,bool,i32,bool,i32),
    pub filters: String,
    pub trigger_pointer: (i32, i32),
    pub filter_lock: bool
}


#[derive(Eq, Clone)]
pub struct Attack
{
    damage: u64,
    victim: String,
    timestamp: String,
    attack_name: String,
    crit: String, // "" for did not crit?
    damage_type: String
}

impl Attack
{
    pub fn attack(&mut self, attack_data: &regex::Captures)
    {
        
    }
    
    pub fn filter(&self, filters: &str) -> bool
    {
        if filters.len() as i32 != 0
        {
            for filter in filters.split_whitespace()
            {
                if !self.timestamp.contains(filter) && !self.victim.contains(filter) && !self.attack_name.contains(filter) && !self.crit.contains(filter) && !self.damage_type.contains(filter) {return false;}
            }
        }
        true
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
        write!(f, "{}\t\tVICTIM: {}\tATTACK: {}\tDAMAGE: {}\tCRIT: {}\tTYPE: {}", self.timestamp, self.victim, self.attack_name, self.damage, self.crit, self.damage_type)
        //write!(f, "{}", self.timestamp, self.victim, self.attack_name)
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
        self.attacks.push(Attack{damage: attack_data.name("damage").unwrap().parse::<u64>().unwrap(), victim: String::from(attack_data.name("target").unwrap()), timestamp: String::from(attack_data.name("datetime").unwrap()), attack_name: String::from(match attack_data.name("attack").unwrap() { "" => "auto attack", val => val } ), crit: String::from(attack_data.name("crittype").unwrap()), damage_type: String::from(attack_data.name("damagetype").unwrap())});
        self.final_damage += attack_data.name("damage").unwrap().parse::<u64>().unwrap();
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
    
    pub fn print_attacks(&self, filters: &str) -> String
    {
        let mut results: String = String::from("");
        for attack in &self.attacks
        {
            if attack.filter(filters)
            {
                results.push_str(&format!("{}\n", attack));
            }
        }
        results
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
        let attacker_name = match attack_data.name("attacker").unwrap() { "" => self.player.as_str(), var => var};
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
        write!(f, "Encounter duration: {}:{}\n", duration.num_minutes(), duration.num_seconds() % 60 );
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
        write!(f, "Encounter duration: {}:{}\n", duration.num_minutes(), duration.num_seconds() % 60 );
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
