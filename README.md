# ACT_linux

Depends on having espeak installed for TTS functionality, however should be able to parse without it.

Depends on xorg-dev through the <a href="https://github.com/aweinstock314/rust-clipboard">clipboard crate.</a>

Depends on ncurses-dev through the <a href="https://github.com/jeaye/ncurses-rs">ncurses crate.</a>

A linux friendly combat parser primarily for EQ2.



Left to do:


*History parsing, parse old encounters. The code for this is essentially done but not in the right order yet.

*HPS parsing, parse heal numbers, the regex cannot yet catch the heals/wards keywords from the log.

*Read triggers from file and save triggers to file.

*Read triggers from the log and place them into a file. (look into Advanced Combat Trackers xml format to possibly make this program compatible.

*Add the possibility to scroll, both in the encounter_win and main_win if the displayed text doesn't fit in that windows height, currently only shows the number of lines that fit.



This is very much a work in progress.

PS. since using regular expressions this program could in the future easilly be expanded to work for other online games that support timestamped logs.
