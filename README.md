# mmo_parser_cli

CURRENTLY IN A BROKEN STATE

Depends on <a href="https://github.com/Morindhal/mmo_parser_backend">mmo_parser_backend</a> where the actual parsing is being done.

Depends on having espeak installed for TTS functionality, however should be able to parse without it.

Depends on xorg-dev through the <a href="https://github.com/aweinstock314/rust-clipboard">clipboard crate.</a>

Depends on ncurses-dev through the <a href="https://github.com/jeaye/ncurses-rs">ncurses crate.</a>

A linux friendly combat parser primarily for EQ2, CLI version.



Left to do:


*HPS parsing, parse heal numbers, the regex cannot yet catch the heals/wards keywords from the log.

*Add the possibility to scroll, both in the encounter_win and main_win if the displayed text doesn't fit in that windows height, currently only shows the number of lines that fit.



This is very much a work in progress.



TODO:
Add logging through the env_logger crate. (errors and such)
Test the newest changes on a real log-file.
Filter out all the small warnings, mostly there due to the naming-convention.
    This is now partly done, the changes need to be tested.
Add error handling code instead of simply unwrapping everything, throw the exceptions into a log.

Add a stable version number when everything is back into a working state. (1.0.0 when heal parsing has been added)
