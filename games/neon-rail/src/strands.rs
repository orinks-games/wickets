//! The archive's strands: the logs around the data core, each a voice with
//! an arc of its own, opened by one kind of play. Text only; `story` holds
//! the types. Shared names: Sable, the skimmer's former pilot; Foreman
//! Hale of the salvage unions; Operator Nine of corporate security; the
//! signal; the origin depot. The courier is never named.

use crate::rules::Route;
use crate::story::{Strand, StrandLog, Unlock};

pub const STRANDS: &[Strand] = &[
    Strand {
        id: "unions",
        name: "The unions",
        blurb: "Foreman Hale's marks on your hull, left along the salvage stretch.",
        logs: &[
            StrandLog {
                title: "First Pass",
                pages: &[
                    "Foreman Hale, cut into your port plate: This stretch is union. We strip here, nobody else does. You took only loose scrap, so you get 1 pass.",
                    "Under your aft hatch I saw the core housing while you idled. That core is what the Authority wants back, and I want to know why. Come back anyway, and bring it.",
                ],
                unlock: Unlock::RouteClears(Route::Salvage, 1),
            },
            StrandLog {
                title: "Maps for Names",
                pages: &[
                    "Foreman Hale, chalk on your aft hatch: Third time through and still you take only loose scrap, nothing of ours. So we trade. Your core knows which haulers came off the rail, and I want that list.",
                    "Payment is scratched under your port plate, 4 turnings. 3 of them nobody runs but us, and the fourth is a lie for anyone who copies your hull. Nothing of mine goes on a network, so your hull is the only copy.",
                ],
                unlock: Unlock::RouteClears(Route::Salvage, 3),
            },
            StrandLog {
                title: "Haulers Not Yet Fallen",
                pages: &[
                    "Foreman Hale, cut deep beside the first mark: Your list reached me. 4 haulers on it have not come off the rail yet, and each has a date beside it. Nobody writes the date of an accident.",
                    "Next derail, my crews stand on the embankment before the hauler falls, and the hold opens first. Scrap was the whole trade until now, and no crew of mine ever cut into a sealed hold. Be at the third turning that night.",
                ],
                unlock: Unlock::RouteClears(Route::Salvage, 6),
            },
        ],
    },
    Strand {
        id: "hunters",
        name: "The hunters",
        blurb: "Sable, who flew this skimmer before you, on the Authority's hunt channel.",
        logs: &[
            StrandLog {
                title: "The Pilot Before You",
                pages: &[
                    "Sable, hunt channel: I know that skimmer better than you do. The stick fights you at full throttle because I bent the mount 2 winters ago. You correct it late, every time.",
                    "Every route you take, I flew first. Tonight I did not follow you through the mixed traffic, I waited at the gap I always used. You came through it 2 seconds after I would have, and 1 second before my lock settled.",
                ],
                unlock: Unlock::RouteClears(Route::Standard, 1),
            },
            StrandLog {
                title: "Where I Stopped",
                pages: &[
                    "Sable, hunt channel: 3 clean nights through the mixed traffic, and the craft gives up my last cockpit entry. It stops at the partition wall. The Authority will tell you I was caught there.",
                    "Cockpit log, Sable. Partition wall at 400 metres, and the gate is standing open with no traffic listed for it. I am cutting the engine, because something is coming through anyway and I want to see it.",
                ],
                unlock: Unlock::RouteClears(Route::Standard, 3),
            },
            StrandLog {
                title: "Not Twice",
                pages: &[
                    "Sable, hunt channel: I had you tonight. My lock tone held on your hull for 4 seconds and I let it drop. Do not count on that twice.",
                    "I took the pardon because only hunters get sent to the wall when the gate opens. 11 times I have watched the same thing come through, and 11 times I kept quiet. You owe the Authority nothing, so get to the wall and look.",
                ],
                unlock: Unlock::RouteClears(Route::Standard, 6),
            },
        ],
    },
    Strand {
        id: "security",
        name: "Security",
        blurb: "Operator Nine's sortie reports from the ambush corridor.",
        logs: &[
            StrandLog {
                title: "Core Intact",
                pages: &[
                    "Operator Nine, sortie 14: the stolen skimmer cleared the eastern artery against 6 drones. All 6 were lost. The data core, the only authorized target, was not destroyed.",
                    "Hull damage to the skimmer was recorded at 3 points, all from debris, none from this unit. The core is carried inside the hull, and no round reaches one without the other. Clarification of the order has been requested.",
                ],
                unlock: Unlock::RouteClears(Route::Ambush, 1),
            },
            StrandLog {
                title: "Losses Accepted",
                pages: &[
                    "Operator Nine, sortie 23: The skimmer has now cleared the eastern artery 3 times against this unit. Drone losses against the skimmer stand at 19. Clarification came back in 8 words, fire on the data core only, losses accepted.",
                    "Authority to disable the craft and recover the core for analysis was requested after sortie 23. Refusal was returned within 4 minutes. No unit is to recover the core, this one included.",
                ],
                unlock: Unlock::RouteClears(Route::Ambush, 3),
            },
            StrandLog {
                title: "Confirmation Pending",
                pages: &[
                    "Operator Nine, sortie 41: The skimmer has cleared the eastern artery 6 times. Drone losses stand at 38. The order to destroy the core only was reissued today, signed for the first time.",
                    "Signatures on the reissued order were checked against the evacuation schedule this desk clears each month. All match. A request for confirmation was filed before sortie 42, and no answer has been received.",
                ],
                unlock: Unlock::RouteClears(Route::Ambush, 6),
            },
        ],
    },
    Strand {
        id: "storm",
        name: "Weather service",
        blurb: "Bulletins still going out from the dark eastern districts, over the storm rail.",
        logs: &[
            StrandLog {
                title: "Evening Bulletin",
                pages: &[
                    "Eastern weather service, evening bulletin. Gusts from the east at 60 kilometres an hour, rain on the storm rail by midnight. The receiver count on this frequency has shown 0 for 26 weeks.",
                    "One skimmer took the storm rail under this station tonight, full speed, no lights. Nothing else has run that rail at night in months. From tonight the bulletin goes out when the skimmer passes.",
                ],
                unlock: Unlock::RouteClears(Route::Storm, 1),
            },
            StrandLog {
                title: "Weather Clearance",
                pages: &[
                    "To the skimmer on the storm rail, from the eastern weather service. Hail after the second relay, rivet sized, gone in 6 minutes. Nobody in the districts is tuned to this frequency, so the bulletin is written for you now.",
                    "Every evacuation train out of the eastern districts asks this station for a weather clearance before the gate opens. I have cleared 31 trains since the sky went black. Not one carried a code for the person at this desk.",
                ],
                unlock: Unlock::RouteClears(Route::Storm, 3),
            },
            StrandLog {
                title: "Gate Forecast",
                pages: &[
                    "Gate forecast, from the eastern weather service, for the skimmer on the storm rail. Pressure on this rail falls hard 40 minutes before the orbital grid stands down, 14 times out of 14. Weather does not keep a schedule, so whatever stands the grid down moves the air on this rail first.",
                    "Partition gates open while the grid is down, so the pressure drop gives you 40 minutes of warning. Next drop is due in 9 days by my count, and this frequency will call it. The roof light is on, 200 metres east of the second relay, and nothing on any schedule stops there.",
                ],
                unlock: Unlock::RouteClears(Route::Storm, 6),
            },
        ],
    },
    Strand {
        id: "signal",
        name: "The signal",
        blurb: "What the network says over the data relays, in transit codes.",
        logs: &[
            StrandLog {
                title: "Hold at Relay",
                pages: &[
                    "The signal: Hold at relay, clear lane. You held at the relay 40 seconds longer than the rail needed, and 3 hunters crossed ahead without turning. The code reached the core 4 seconds before their drives came into range.",
                    "The signal: Hold at relay, clear lane, go. The core went back through the 9 nights the hunters lost you. The first 2 codes sit in the record before every one of them.",
                ],
                unlock: Unlock::RouteClears(Route::Relay, 1),
            },
            StrandLog {
                title: "3 Nights Early",
                pages: &[
                    "The signal: Divert at second relay, hold 60 seconds, lane clear. The code reached the core while you were 2 districts out, on a rail that never touches this relay. You took the diversion when it opened, and 4 hunter drives passed the relay you had left.",
                    "The signal: Route held, divert at second relay, hold 60 seconds, lane clear. The core matched the code against its record and found it 3 nights old. You had not flown tonight's rail 3 nights ago, and the code was already waiting on it.",
                ],
                unlock: Unlock::RouteClears(Route::Relay, 3),
            },
            StrandLog {
                title: "Origin Dispatch",
                pages: &[
                    "The signal: Dispatch, origin depot, all reachable traffic hold for routing, lane clear to origin. The core had no record of that dispatch stamp before tonight. It belongs to the depot that built this skimmer, and the depot is still issuing routes.",
                    "The signal: Dispatch, origin depot, to build 41, acknowledge, route held to origin, return to origin. Build 41 is this skimmer's number, stamped on the frame where no pilot looks. The dispatch clock reads tonight's hour exactly, and no skimmer has run into that depot since the partition.",
                ],
                unlock: Unlock::RouteClears(Route::Relay, 6),
            },
        ],
    },
    Strand {
        id: "blackbox",
        name: "Black box",
        blurb: "What the Authority's recovery crews write up after your wrecks.",
        logs: &[
            StrandLog {
                title: "Nobody Touched It",
                pages: &[
                    "Transit Authority recovery crew, first tag on this craft. Stolen combat skimmer, found 300 metres past a dead gate, nose down in the ballast. Hull torn open on the port side, 3 drone hits, pilot not at the site.",
                    "Core intact, housing scorched, warm through the glove. At 31 minutes of a 40 minute crane wait, the dead gate took power and the craft ran. Crew chief: filed as gone from the site, and nobody on this crew touched it.",
                ],
                unlock: Unlock::Failures(1),
            },
            StrandLog {
                title: "Cutter on the Core",
                pages: &[
                    "Third tag on this craft, same crew, same core. Found upside down in a drainage cut under 2 wrecked haulers. Hull split end to end, core warm in its housing.",
                    "Cutter on the housing 9 minutes, discharge clamp 10 minutes, no mark and no drop. At 19 minutes the rail took power under 2 haulers and 1 skimmer, and only the skimmer moved. Crew chief: the craft crashes and the core does not, and nobody powers a rail for 2 wrecked haulers.",
                ],
                unlock: Unlock::Failures(3),
            },
            StrandLog {
                title: "Power From Nowhere",
                pages: &[
                    "Seventh tag on this craft, and this time we pulled the rail log before the craft could leave. Power to the siding was ordered 12 minutes after the crash, by transit code, from no station. Cockpit log shows 2 pilots on this craft, and the only callsign in it is Sable.",
                    "Tags 1 through 6 are still in this craft's log, each one opened. Whoever sits in that seat reads what we write, so this line is for the seat. Crew chief: pilot, nobody on this crew puts you back on the rail, and no station does either.",
                ],
                unlock: Unlock::Failures(7),
            },
            StrandLog {
                title: "Left Where It Lies",
                pages: &[
                    "Twelfth tag on this craft, written at the site with 2 drones still circling. Found short of the partition wall, hull open, 14 drone hits on the core housing, none through. Core intact, warm, as it was the other 11 times.",
                    "11 tags went up the line, and every one told the drones where to find the core. We leave it where it lies, core and all, and this tag goes nowhere but the craft. Crew chief: 11 times somebody put this back on the rail while we counted, and we are done counting.",
                ],
                unlock: Unlock::Failures(12),
            },
        ],
    },
    Strand {
        id: "deeprail",
        name: "Deep rail",
        blurb: "The stations under the partition, deep on the endless rail.",
        logs: &[
            StrandLog {
                title: "Still On Time",
                pages: &[
                    "Station announcement, sector 4: The next service to the eastern districts arrives in 4 minutes. Stand behind the yellow line. The skimmer records the announcement 3 times while passing 1 platform.",
                    "Platform sign, sector 4: All services running on time. The clock beside the sign has not moved since the orbital grid stood down. By that clock, every train the deep stations promise is on time.",
                ],
                unlock: Unlock::EndlessSector(4),
            },
            StrandLog {
                title: "Behind the Yellow Line",
                pages: &[
                    "Station announcement, sector 7: This service is full, please wait for the next train. The skimmer counts 40 people on the platform, all behind the yellow line. None of them step forward when the skimmer passes.",
                    "Platform sign, sector 7: Priority boarding only, clean transit codes required. Nobody waiting here broadcasts a code. The sign has not changed since the last full train pulled out of here.",
                ],
                unlock: Unlock::EndlessSector(7),
            },
            StrandLog {
                title: "Rewritten Boards",
                pages: &[
                    "Station announcement, sector 10: The next service has been delayed. It will now depart during the 90 second stand down. The recording has no hiss on it, and every other deep announcement does.",
                    "Platform sign, sector 10: Passengers without transit codes may board here. The letters are new and carry no dust. Under them, the old sign still says priority boarding only.",
                ],
                unlock: Unlock::EndlessSector(10),
            },
            StrandLog {
                title: "One Lit Platform",
                pages: &[
                    "Station announcement, sector 14: The service to the origin depot is boarding now. No other platform in the deep sectors has its lights on. The people here have stepped over the yellow line.",
                    "Platform sign, sector 14: Destination, the origin depot. The vehicle on the board is a skimmer, and the hull number is yours. Departure in 90 seconds, and nobody has run a skimmer into the origin depot since the partition.",
                ],
                unlock: Unlock::EndlessSector(14),
            },
        ],
    },
    Strand {
        id: "hangar",
        name: "Hangar",
        blurb: "Your own notes in the skimmer's maintenance log.",
        logs: &[
            StrandLog {
                title: "Same Wall",
                pages: &[
                    "10 runs now on a craft nobody handed me, and 2 rail pins swapped tonight. My own route was 14 years on the same 6 stations, parcels and parts. The partition cut it at the wall, and the licence went with it on the same form.",
                    "Stealing is the word the Transit Authority uses, and it is the right one. The hangar had stood unlocked since the partition, and the craft inside still had fuel. Its cockpit log keeps the last route before mine, and that route also ends at the partition wall.",
                ],
                unlock: Unlock::RunsFinished(10),
            },
            StrandLog {
                title: "Full Every Dawn",
                pages: &[
                    "25 runs and the city has a shape I did not see from my old route. Each rail that still keeps time runs away from the eastern districts, and none runs back. Priority traffic gets a clear track through any weather, and the rest of us get what it leaves.",
                    "Fuel is the part I cannot account for. I logged the gauge at night and at dawn for 6 nights, and every dawn it showed full. Somebody comes in while I sleep and leaves no mark, so somebody wants these runs flown.",
                ],
                unlock: Unlock::RunsFinished(25),
            },
            StrandLog {
                title: "Sortie Numbers",
                pages: &[
                    "50 drones down, and the hunt channel calls that count an ace. The last licence I held was for parcels. I pulled a shard of the fiftieth from my own plating with pliers, and it carried a sortie number.",
                    "Operator Nine is the mark beside the sortie number, stamped before the drone left its rack. Every hit this craft has taken landed on the core bay and left the cabin alone. 50 drones is 50 sorties somebody wrote up with the target named, then went home for the night.",
                ],
                unlock: Unlock::Kills(50),
            },
            StrandLog {
                title: "Build Number",
                pages: &[
                    "Build 41 is what the hunt channel calls this craft now. They say it the way they say sleet or a blocked junction, a condition on the rail. The tank was full again this morning, and I have stopped checking the door.",
                    "Next run is mine to choose, the first choice since the licence went. Nobody has run a skimmer back into the origin depot since the partition, and this one was built there. I am going to see whether a full tank reaches it, and who has been filling the tank.",
                ],
                unlock: Unlock::Rank(5),
            },
        ],
    },
    Strand {
        id: "origin",
        name: "Origin",
        blurb: "Where the rails lead once the data core is complete.",
        logs: &[
            StrandLog {
                title: "Off the Map",
                pages: &[
                    "Extraction clears and the signal does not release you. The signal: hold heading, switch at the partition wall. Your skimmer takes a switch no map shows and runs past the wall toward the origin depot.",
                    "Behind you, 3 hunters slow and stop at the partition wall. Their craft run Transit Authority maps, and the Transit Authority has no rail here. Sable's last route ends at this wall in the cockpit logs, but the rail keeps going.",
                ],
                unlock: Unlock::OriginRuns(1),
            },
            StrandLog {
                title: "Every Relay at Once",
                pages: &[
                    "Depot gates, sealed since the partition, scan your hull before you reach them. The origin depot asks for a build number, and the skimmer answers with its own. The first gate opens, then the second, and the rail does not slow.",
                    "Every relay in Black Sky City speaks at once, the moment the gates open. The schedule plays over the whole network, and every signature goes out with it. Operator Nine's drones are tuned to destroy 1 data core, and the city now holds thousands of copies.",
                ],
                unlock: Unlock::OriginRuns(2),
            },
            StrandLog {
                title: "Trains Both Ways",
                pages: &[
                    "Rails open across Black Sky City, every gate at once, and nobody is asked for a clean transit code. Evacuation trains run both ways now, into the eastern districts and back out, and nobody aboard is sedated. Sable speaks on the open channel to every hunter in the air: pursuit ended, stand down.",
                    "The signal: all lanes clear, land at origin. After that code the signal goes quiet, and the origin depot ahead is dark, unmapped even by Foreman Hale. Your skimmer sets down inside, its running lights the only light in the depot, and the engines stop.",
                ],
                unlock: Unlock::OriginRuns(3),
            },
        ],
    },
];
