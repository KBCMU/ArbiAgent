//! Shared team name ↔ abbreviation dictionary for cross-platform matching.
//!
//! Provides bidirectional lookup: full/partial team names → canonical abbreviation,
//! and abbreviation → primary full name. Organized by league so new sports can be
//! added by appending a block of entries.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::models::event::Sport;

/// A single entry mapping one or more keywords to a canonical abbreviation.
struct TeamEntry {
    abbrev: &'static str,
    keywords: &'static [&'static str],
    sport: Sport,
}

/// All team entries across all supported leagues.
static TEAM_ENTRIES: &[TeamEntry] = &[
    // ── NBA ──────────────────────────────────────────────────────────
    TeamEntry { abbrev: "ATL", keywords: &["hawks", "atlanta hawks", "atlanta"], sport: Sport::Nba },
    TeamEntry { abbrev: "BOS", keywords: &["celtics", "boston celtics", "boston"], sport: Sport::Nba },
    TeamEntry { abbrev: "BKN", keywords: &["nets", "brooklyn nets", "brooklyn", "bk", "brk"], sport: Sport::Nba },
    TeamEntry { abbrev: "CHA", keywords: &["hornets", "charlotte hornets", "charlotte"], sport: Sport::Nba },
    TeamEntry { abbrev: "CHI", keywords: &["bulls", "chicago bulls", "chicago"], sport: Sport::Nba },
    TeamEntry { abbrev: "CLE", keywords: &["cavaliers", "cavs", "cleveland cavaliers", "cleveland"], sport: Sport::Nba },
    TeamEntry { abbrev: "DAL", keywords: &["mavericks", "mavs", "dallas mavericks"], sport: Sport::Nba },
    TeamEntry { abbrev: "DEN", keywords: &["nuggets", "denver nuggets", "denver"], sport: Sport::Nba },
    TeamEntry { abbrev: "DET", keywords: &["pistons", "detroit pistons"], sport: Sport::Nba },
    TeamEntry { abbrev: "GSW", keywords: &["warriors", "golden state warriors", "golden state", "gs"], sport: Sport::Nba },
    TeamEntry { abbrev: "HOU", keywords: &["rockets", "houston rockets", "houston"], sport: Sport::Nba },
    TeamEntry { abbrev: "IND", keywords: &["pacers", "indiana pacers"], sport: Sport::Nba },
    TeamEntry { abbrev: "LAC", keywords: &["clippers", "la clippers", "los angeles clippers"], sport: Sport::Nba },
    TeamEntry { abbrev: "LAL", keywords: &["lakers", "la lakers", "los angeles lakers"], sport: Sport::Nba },
    TeamEntry { abbrev: "MEM", keywords: &["grizzlies", "memphis grizzlies", "memphis"], sport: Sport::Nba },
    TeamEntry { abbrev: "MIA", keywords: &["heat", "miami heat"], sport: Sport::Nba },
    TeamEntry { abbrev: "MIL", keywords: &["bucks", "milwaukee bucks", "milwaukee"], sport: Sport::Nba },
    TeamEntry { abbrev: "MIN", keywords: &["timberwolves", "wolves", "minnesota timberwolves"], sport: Sport::Nba },
    TeamEntry { abbrev: "NOP", keywords: &["pelicans", "new orleans pelicans", "new orleans", "no"], sport: Sport::Nba },
    TeamEntry { abbrev: "NYK", keywords: &["knicks", "new york knicks"], sport: Sport::Nba },
    TeamEntry { abbrev: "OKC", keywords: &["thunder", "oklahoma city thunder", "oklahoma city"], sport: Sport::Nba },
    TeamEntry { abbrev: "ORL", keywords: &["magic", "orlando magic"], sport: Sport::Nba },
    TeamEntry { abbrev: "PHI", keywords: &["76ers", "sixers", "philadelphia 76ers", "philadelphia"], sport: Sport::Nba },
    TeamEntry { abbrev: "PHX", keywords: &["suns", "phoenix suns", "phoenix", "pho"], sport: Sport::Nba },
    TeamEntry { abbrev: "POR", keywords: &["trail blazers", "blazers", "portland trail blazers", "portland"], sport: Sport::Nba },
    TeamEntry { abbrev: "SAC", keywords: &["kings", "sacramento kings", "sacramento"], sport: Sport::Nba },
    TeamEntry { abbrev: "SAS", keywords: &["spurs", "san antonio spurs", "san antonio", "sa"], sport: Sport::Nba },
    TeamEntry { abbrev: "TOR", keywords: &["raptors", "toronto raptors", "toronto"], sport: Sport::Nba },
    TeamEntry { abbrev: "UTA", keywords: &["jazz", "utah jazz", "utah"], sport: Sport::Nba },
    TeamEntry { abbrev: "WAS", keywords: &["wizards", "washington wizards"], sport: Sport::Nba },

    // ── NFL ──────────────────────────────────────────────────────────
    TeamEntry { abbrev: "ARI", keywords: &["cardinals", "arizona cardinals", "arizona"], sport: Sport::Nfl },
    TeamEntry { abbrev: "ATL", keywords: &["falcons", "atlanta falcons"], sport: Sport::Nfl },
    TeamEntry { abbrev: "BAL", keywords: &["ravens", "baltimore ravens", "baltimore"], sport: Sport::Nfl },
    TeamEntry { abbrev: "BUF", keywords: &["bills", "buffalo bills", "buffalo"], sport: Sport::Nfl },
    TeamEntry { abbrev: "CAR", keywords: &["panthers", "carolina panthers", "carolina"], sport: Sport::Nfl },
    TeamEntry { abbrev: "CHI", keywords: &["bears", "chicago bears"], sport: Sport::Nfl },
    TeamEntry { abbrev: "CIN", keywords: &["bengals", "cincinnati bengals", "cincinnati"], sport: Sport::Nfl },
    TeamEntry { abbrev: "CLE", keywords: &["browns", "cleveland browns"], sport: Sport::Nfl },
    TeamEntry { abbrev: "DAL", keywords: &["cowboys", "dallas cowboys"], sport: Sport::Nfl },
    TeamEntry { abbrev: "DEN", keywords: &["broncos", "denver broncos"], sport: Sport::Nfl },
    TeamEntry { abbrev: "DET", keywords: &["lions", "detroit lions"], sport: Sport::Nfl },
    TeamEntry { abbrev: "GB",  keywords: &["packers", "green bay packers", "green bay"], sport: Sport::Nfl },
    TeamEntry { abbrev: "HOU", keywords: &["texans", "houston texans"], sport: Sport::Nfl },
    TeamEntry { abbrev: "IND", keywords: &["colts", "indianapolis colts", "indianapolis"], sport: Sport::Nfl },
    TeamEntry { abbrev: "JAX", keywords: &["jaguars", "jacksonville jaguars", "jacksonville", "jac"], sport: Sport::Nfl },
    TeamEntry { abbrev: "KC",  keywords: &["chiefs", "kansas city chiefs", "kansas city"], sport: Sport::Nfl },
    TeamEntry { abbrev: "LAC", keywords: &["chargers", "la chargers", "los angeles chargers"], sport: Sport::Nfl },
    TeamEntry { abbrev: "LAR", keywords: &["rams", "la rams", "los angeles rams"], sport: Sport::Nfl },
    TeamEntry { abbrev: "LV",  keywords: &["raiders", "las vegas raiders", "las vegas"], sport: Sport::Nfl },
    TeamEntry { abbrev: "MIA", keywords: &["dolphins", "miami dolphins"], sport: Sport::Nfl },
    TeamEntry { abbrev: "MIN", keywords: &["vikings", "minnesota vikings"], sport: Sport::Nfl },
    TeamEntry { abbrev: "NE",  keywords: &["patriots", "new england patriots", "new england"], sport: Sport::Nfl },
    TeamEntry { abbrev: "NO",  keywords: &["saints", "new orleans saints"], sport: Sport::Nfl },
    TeamEntry { abbrev: "NYG", keywords: &["giants", "new york giants", "ny giants"], sport: Sport::Nfl },
    TeamEntry { abbrev: "NYJ", keywords: &["jets", "new york jets", "ny jets"], sport: Sport::Nfl },
    TeamEntry { abbrev: "PHI", keywords: &["eagles", "philadelphia eagles"], sport: Sport::Nfl },
    TeamEntry { abbrev: "PIT", keywords: &["steelers", "pittsburgh steelers", "pittsburgh"], sport: Sport::Nfl },
    TeamEntry { abbrev: "SEA", keywords: &["seahawks", "seattle seahawks", "seattle"], sport: Sport::Nfl },
    TeamEntry { abbrev: "SF",  keywords: &["49ers", "niners", "san francisco 49ers", "san francisco"], sport: Sport::Nfl },
    TeamEntry { abbrev: "TB",  keywords: &["buccaneers", "bucs", "tampa bay buccaneers", "tampa bay"], sport: Sport::Nfl },
    TeamEntry { abbrev: "TEN", keywords: &["titans", "tennessee titans", "tennessee"], sport: Sport::Nfl },
    TeamEntry { abbrev: "WAS", keywords: &["commanders", "washington commanders"], sport: Sport::Nfl },

    // ── MLB ──────────────────────────────────────────────────────────
    TeamEntry { abbrev: "ARI", keywords: &["diamondbacks", "d-backs", "arizona diamondbacks"], sport: Sport::Mlb },
    TeamEntry { abbrev: "ATL", keywords: &["braves", "atlanta braves"], sport: Sport::Mlb },
    TeamEntry { abbrev: "BAL", keywords: &["orioles", "baltimore orioles"], sport: Sport::Mlb },
    TeamEntry { abbrev: "BOS", keywords: &["red sox", "boston red sox"], sport: Sport::Mlb },
    TeamEntry { abbrev: "CHC", keywords: &["cubs", "chicago cubs"], sport: Sport::Mlb },
    TeamEntry { abbrev: "CHW", keywords: &["white sox", "chicago white sox", "cws"], sport: Sport::Mlb },
    TeamEntry { abbrev: "CIN", keywords: &["reds", "cincinnati reds"], sport: Sport::Mlb },
    TeamEntry { abbrev: "CLE", keywords: &["guardians", "cleveland guardians"], sport: Sport::Mlb },
    TeamEntry { abbrev: "COL", keywords: &["rockies", "colorado rockies", "colorado"], sport: Sport::Mlb },
    TeamEntry { abbrev: "DET", keywords: &["tigers", "detroit tigers"], sport: Sport::Mlb },
    TeamEntry { abbrev: "HOU", keywords: &["astros", "houston astros"], sport: Sport::Mlb },
    TeamEntry { abbrev: "KC",  keywords: &["royals", "kansas city royals"], sport: Sport::Mlb },
    TeamEntry { abbrev: "LAA", keywords: &["angels", "los angeles angels", "la angels"], sport: Sport::Mlb },
    TeamEntry { abbrev: "LAD", keywords: &["dodgers", "los angeles dodgers", "la dodgers"], sport: Sport::Mlb },
    TeamEntry { abbrev: "MIA", keywords: &["marlins", "miami marlins"], sport: Sport::Mlb },
    TeamEntry { abbrev: "MIL", keywords: &["brewers", "milwaukee brewers"], sport: Sport::Mlb },
    TeamEntry { abbrev: "MIN", keywords: &["twins", "minnesota twins"], sport: Sport::Mlb },
    TeamEntry { abbrev: "NYM", keywords: &["mets", "new york mets", "ny mets"], sport: Sport::Mlb },
    TeamEntry { abbrev: "NYY", keywords: &["yankees", "new york yankees", "ny yankees"], sport: Sport::Mlb },
    TeamEntry { abbrev: "OAK", keywords: &["athletics", "oakland athletics", "oakland", "a's"], sport: Sport::Mlb },
    TeamEntry { abbrev: "PHI", keywords: &["phillies", "philadelphia phillies"], sport: Sport::Mlb },
    TeamEntry { abbrev: "PIT", keywords: &["pirates", "pittsburgh pirates"], sport: Sport::Mlb },
    TeamEntry { abbrev: "SD",  keywords: &["padres", "san diego padres", "san diego"], sport: Sport::Mlb },
    TeamEntry { abbrev: "SEA", keywords: &["mariners", "seattle mariners"], sport: Sport::Mlb },
    TeamEntry { abbrev: "SF",  keywords: &["giants", "san francisco giants"], sport: Sport::Mlb },
    TeamEntry { abbrev: "STL", keywords: &["cardinals", "st. louis cardinals", "st louis cardinals", "st louis"], sport: Sport::Mlb },
    TeamEntry { abbrev: "TB",  keywords: &["rays", "tampa bay rays"], sport: Sport::Mlb },
    TeamEntry { abbrev: "TEX", keywords: &["rangers", "texas rangers", "texas"], sport: Sport::Mlb },
    TeamEntry { abbrev: "TOR", keywords: &["blue jays", "toronto blue jays"], sport: Sport::Mlb },
    TeamEntry { abbrev: "WAS", keywords: &["nationals", "washington nationals", "nats"], sport: Sport::Mlb },

    // ── NHL ──────────────────────────────────────────────────────────
    // Keywords include Kalshi ticker abbreviations (e.g. "nj" for NJD) so both
    // platforms normalize to the same canonical abbreviation.
    TeamEntry { abbrev: "ANA", keywords: &["ducks", "anaheim ducks", "anaheim"], sport: Sport::Nhl },
    TeamEntry { abbrev: "UTA", keywords: &["utah hockey club", "utah hc", "utah hockey"], sport: Sport::Nhl },
    TeamEntry { abbrev: "BOS", keywords: &["bruins", "boston bruins"], sport: Sport::Nhl },
    TeamEntry { abbrev: "BUF", keywords: &["sabres", "buffalo sabres"], sport: Sport::Nhl },
    TeamEntry { abbrev: "CGY", keywords: &["flames", "calgary flames", "calgary"], sport: Sport::Nhl },
    TeamEntry { abbrev: "CAR", keywords: &["hurricanes", "carolina hurricanes", "canes"], sport: Sport::Nhl },
    TeamEntry { abbrev: "CHI", keywords: &["blackhawks", "chicago blackhawks"], sport: Sport::Nhl },
    TeamEntry { abbrev: "COL", keywords: &["avalanche", "colorado avalanche", "avs"], sport: Sport::Nhl },
    TeamEntry { abbrev: "CBJ", keywords: &["blue jackets", "columbus blue jackets", "columbus"], sport: Sport::Nhl },
    TeamEntry { abbrev: "DAL", keywords: &["stars", "dallas stars"], sport: Sport::Nhl },
    TeamEntry { abbrev: "DET", keywords: &["red wings", "detroit red wings"], sport: Sport::Nhl },
    TeamEntry { abbrev: "EDM", keywords: &["oilers", "edmonton oilers", "edmonton"], sport: Sport::Nhl },
    TeamEntry { abbrev: "FLA", keywords: &["panthers", "florida panthers", "florida"], sport: Sport::Nhl },
    TeamEntry { abbrev: "LAK", keywords: &["kings", "la kings", "los angeles kings", "lak"], sport: Sport::Nhl },
    TeamEntry { abbrev: "MIN", keywords: &["wild", "minnesota wild"], sport: Sport::Nhl },
    TeamEntry { abbrev: "MTL", keywords: &["canadiens", "montreal canadiens", "montreal", "habs", "mon"], sport: Sport::Nhl },
    TeamEntry { abbrev: "NSH", keywords: &["predators", "nashville predators", "nashville", "preds"], sport: Sport::Nhl },
    TeamEntry { abbrev: "NJD", keywords: &["devils", "new jersey devils", "new jersey", "nj"], sport: Sport::Nhl },
    TeamEntry { abbrev: "NYI", keywords: &["islanders", "new york islanders", "ny islanders"], sport: Sport::Nhl },
    TeamEntry { abbrev: "NYR", keywords: &["rangers", "new york rangers", "ny rangers"], sport: Sport::Nhl },
    TeamEntry { abbrev: "OTT", keywords: &["senators", "ottawa senators", "ottawa", "sens"], sport: Sport::Nhl },
    TeamEntry { abbrev: "PHI", keywords: &["flyers", "philadelphia flyers"], sport: Sport::Nhl },
    TeamEntry { abbrev: "PIT", keywords: &["penguins", "pittsburgh penguins", "pens"], sport: Sport::Nhl },
    TeamEntry { abbrev: "SJS", keywords: &["sharks", "san jose sharks", "san jose", "sj"], sport: Sport::Nhl },
    TeamEntry { abbrev: "SEA", keywords: &["kraken", "seattle kraken"], sport: Sport::Nhl },
    TeamEntry { abbrev: "STL", keywords: &["blues", "st. louis blues", "st louis blues"], sport: Sport::Nhl },
    TeamEntry { abbrev: "TB",  keywords: &["lightning", "tampa bay lightning", "bolts", "tbl"], sport: Sport::Nhl },
    TeamEntry { abbrev: "TOR", keywords: &["maple leafs", "toronto maple leafs", "leafs"], sport: Sport::Nhl },
    TeamEntry { abbrev: "VAN", keywords: &["canucks", "vancouver canucks", "vancouver"], sport: Sport::Nhl },
    TeamEntry { abbrev: "VGK", keywords: &["golden knights", "vegas golden knights", "vegas", "vgs"], sport: Sport::Nhl },
    TeamEntry { abbrev: "WPG", keywords: &["jets", "winnipeg jets", "winnipeg"], sport: Sport::Nhl },
    TeamEntry { abbrev: "WSH", keywords: &["capitals", "washington capitals", "caps", "was"], sport: Sport::Nhl },

    // ── CBB (College Basketball — major programs) ────────────────────
    TeamEntry { abbrev: "DUK", keywords: &["duke", "duke blue devils", "blue devils"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UNC", keywords: &["north carolina", "unc", "tar heels", "tarheels"], sport: Sport::Cbb },
    TeamEntry { abbrev: "KU", keywords: &["kansas", "kansas jayhawks", "jayhawks"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UK", keywords: &["kentucky", "kentucky wildcats"], sport: Sport::Cbb },
    TeamEntry { abbrev: "GONZ", keywords: &["gonzaga", "gonzaga bulldogs", "zags"], sport: Sport::Cbb },
    TeamEntry { abbrev: "AUB", keywords: &["auburn", "auburn tigers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "HOU", keywords: &["houston", "houston cougars"], sport: Sport::Cbb },
    TeamEntry { abbrev: "FLA", keywords: &["florida", "florida gators", "gators"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TENN", keywords: &["tennessee", "tennessee volunteers", "vols"], sport: Sport::Cbb },
    TeamEntry { abbrev: "PUR", keywords: &["purdue", "purdue boilermakers", "boilermakers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ALA", keywords: &["alabama", "alabama crimson tide", "crimson tide", "bama"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ARIZ", keywords: &["arizona", "arizona wildcats"], sport: Sport::Cbb },
    TeamEntry { abbrev: "BAYLOR", keywords: &["baylor", "baylor bears"], sport: Sport::Cbb },
    TeamEntry { abbrev: "CONN", keywords: &["uconn", "connecticut", "connecticut huskies", "huskies"], sport: Sport::Cbb },
    TeamEntry { abbrev: "IOWA", keywords: &["iowa", "iowa hawkeyes", "hawkeyes"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ISU", keywords: &["iowa state", "iowa state cyclones", "cyclones"], sport: Sport::Cbb },
    TeamEntry { abbrev: "IU", keywords: &["indiana", "indiana hoosiers", "hoosiers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MARQ", keywords: &["marquette", "marquette golden eagles"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MICH", keywords: &["michigan", "michigan wolverines", "wolverines"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MSU", keywords: &["michigan state", "michigan state spartans", "spartans"], sport: Sport::Cbb },
    TeamEntry { abbrev: "NCST", keywords: &["nc state", "north carolina state", "wolfpack"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ORE", keywords: &["oregon", "oregon ducks"], sport: Sport::Cbb },
    TeamEntry { abbrev: "OSU", keywords: &["ohio state", "ohio state buckeyes", "buckeyes"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SYR", keywords: &["syracuse", "syracuse orange"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TEX", keywords: &["texas", "texas longhorns", "longhorns"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TXAM", keywords: &["texas a&m", "texas am", "aggies"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UCLA", keywords: &["ucla", "ucla bruins", "bruins"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UVA", keywords: &["virginia", "virginia cavaliers", "cavaliers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "VILL", keywords: &["villanova", "villanova wildcats"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WIS", keywords: &["wisconsin", "wisconsin badgers", "badgers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "CREIGH", keywords: &["creighton", "creighton bluejays"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SDSU", keywords: &["san diego state", "san diego state aztecs", "aztecs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MIZZ", keywords: &["missouri", "missouri tigers", "mizzou"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ARK", keywords: &["arkansas", "arkansas razorbacks", "razorbacks"], sport: Sport::Cbb },
    TeamEntry { abbrev: "LSU", keywords: &["lsu", "louisiana state", "lsu tigers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MISS", keywords: &["ole miss", "mississippi", "mississippi rebels", "rebels"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SC", keywords: &["south carolina", "south carolina gamecocks", "gamecocks"], sport: Sport::Cbb },
    TeamEntry { abbrev: "VT", keywords: &["virginia tech", "virginia tech hokies", "hokies"], sport: Sport::Cbb },
    TeamEntry { abbrev: "LOU", keywords: &["louisville", "louisville cardinals"], sport: Sport::Cbb },
    TeamEntry { abbrev: "PITT", keywords: &["pittsburgh", "pittsburgh panthers", "pitt"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WAKE", keywords: &["wake forest", "wake forest demon deacons"], sport: Sport::Cbb },
    TeamEntry { abbrev: "CLEM", keywords: &["clemson", "clemson tigers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "GT", keywords: &["georgia tech", "georgia tech yellow jackets"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UGA", keywords: &["georgia", "georgia bulldogs", "bulldogs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TTU", keywords: &["texas tech", "texas tech red raiders", "red raiders"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TCU", keywords: &["tcu", "texas christian"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WVU", keywords: &["west virginia", "west virginia mountaineers", "mountaineers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "OKLA", keywords: &["oklahoma", "oklahoma sooners", "sooners"], sport: Sport::Cbb },
    TeamEntry { abbrev: "OKST", keywords: &["oklahoma state", "oklahoma state cowboys"], sport: Sport::Cbb },
    TeamEntry { abbrev: "STAN", keywords: &["stanford", "stanford cardinal"], sport: Sport::Cbb },
    TeamEntry { abbrev: "USC", keywords: &["usc", "southern california", "trojans"], sport: Sport::Cbb },
    TeamEntry { abbrev: "COLO", keywords: &["colorado", "colorado buffaloes", "buffaloes", "buffs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UTAH", keywords: &["utah", "utah utes", "utes"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ASU", keywords: &["arizona state", "arizona state sun devils", "sun devils"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WASH", keywords: &["washington", "washington huskies"], sport: Sport::Cbb },
    TeamEntry { abbrev: "NEB", keywords: &["nebraska", "nebraska cornhuskers", "cornhuskers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MINN", keywords: &["minnesota", "minnesota golden gophers", "golden gophers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "ILL", keywords: &["illinois", "illinois fighting illini", "illini"], sport: Sport::Cbb },
    TeamEntry { abbrev: "NW", keywords: &["northwestern", "northwestern wildcats"], sport: Sport::Cbb },
    TeamEntry { abbrev: "RUT", keywords: &["rutgers", "rutgers scarlet knights"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MARY", keywords: &["maryland", "maryland terrapins", "terps"], sport: Sport::Cbb },
    TeamEntry { abbrev: "PSU", keywords: &["penn state", "penn state nittany lions", "nittany lions"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SMC", keywords: &["saint mary's", "saint marys", "st mary's", "st marys", "gaels"], sport: Sport::Cbb },
    TeamEntry { abbrev: "DAY", keywords: &["dayton", "dayton flyers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MEMPH", keywords: &["memphis", "memphis tigers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "CINCY", keywords: &["cincinnati", "cincinnati bearcats", "bearcats"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MSST", keywords: &["mississippi state", "mississippi state bulldogs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WEBB", keywords: &["gardner-webb", "gardner webb", "runnin' bulldogs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TROY", keywords: &["troy", "troy trojans"], sport: Sport::Cbb },
    TeamEntry { abbrev: "NOVA", keywords: &["nova southeastern", "nova se"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SJSU", keywords: &["san jose state", "san jose state spartans"], sport: Sport::Cbb },
    TeamEntry { abbrev: "NMST", keywords: &["new mexico state", "new mexico state aggies"], sport: Sport::Cbb },
    TeamEntry { abbrev: "NMX", keywords: &["new mexico", "new mexico lobos", "lobos"], sport: Sport::Cbb },
    TeamEntry { abbrev: "USF", keywords: &["south florida", "south florida bulls"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UTEP", keywords: &["utep", "texas el paso", "miners"], sport: Sport::Cbb },
    TeamEntry { abbrev: "RICE", keywords: &["rice", "rice owls"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SMU", keywords: &["smu", "southern methodist", "mustangs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TULN", keywords: &["tulane", "tulane green wave", "green wave"], sport: Sport::Cbb },
    TeamEntry { abbrev: "TULSA", keywords: &["tulsa", "tulsa golden hurricane"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WICH", keywords: &["wichita state", "wichita state shockers", "shockers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "DRAKE", keywords: &["drake", "drake bulldogs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "SETON", keywords: &["seton hall", "seton hall pirates"], sport: Sport::Cbb },
    TeamEntry { abbrev: "GTOWN", keywords: &["georgetown", "georgetown hoyas", "hoyas"], sport: Sport::Cbb },
    TeamEntry { abbrev: "PROV", keywords: &["providence", "providence friars", "friars"], sport: Sport::Cbb },
    TeamEntry { abbrev: "DEPAUL", keywords: &["depaul", "depaul blue demons"], sport: Sport::Cbb },
    TeamEntry { abbrev: "XAVI", keywords: &["xavier", "xavier musketeers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "BUTLR", keywords: &["butler", "butler bulldogs"], sport: Sport::Cbb },
    TeamEntry { abbrev: "VCU", keywords: &["vcu", "virginia commonwealth", "rams"], sport: Sport::Cbb },
    TeamEntry { abbrev: "RICH", keywords: &["richmond", "richmond spiders", "spiders"], sport: Sport::Cbb },
    TeamEntry { abbrev: "STBON", keywords: &["st bonaventure", "saint bonaventure", "bonnies"], sport: Sport::Cbb },
    TeamEntry { abbrev: "GEORGE", keywords: &["george mason", "george mason patriots"], sport: Sport::Cbb },
    TeamEntry { abbrev: "FAIR", keywords: &["fairleigh dickinson", "fdu"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UNT", keywords: &["north texas", "north texas mean green", "mean green"], sport: Sport::Cbb },
    TeamEntry { abbrev: "MARSH", keywords: &["marshall", "marshall thundering herd"], sport: Sport::Cbb },
    TeamEntry { abbrev: "UAB", keywords: &["uab", "alabama birmingham", "blazers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "WKU", keywords: &["western kentucky", "western kentucky hilltoppers", "hilltoppers"], sport: Sport::Cbb },
    TeamEntry { abbrev: "BYU", keywords: &["byu", "brigham young"], sport: Sport::Cbb },
    TeamEntry { abbrev: "KSU", keywords: &["kansas state", "kansas state wildcats"], sport: Sport::Cbb },
    TeamEntry { abbrev: "VANDY", keywords: &["vanderbilt", "vanderbilt commodores", "commodores"], sport: Sport::Cbb },

    // ── CFB (College Football — major programs) ─────────────────────
    TeamEntry { abbrev: "DUK", keywords: &["duke", "duke blue devils"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UNC", keywords: &["north carolina", "unc", "tar heels"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ALA", keywords: &["alabama", "crimson tide", "bama"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UGA", keywords: &["georgia", "georgia bulldogs", "bulldogs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "OSU", keywords: &["ohio state", "ohio state buckeyes", "buckeyes"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MICH", keywords: &["michigan", "michigan wolverines", "wolverines"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TEX", keywords: &["texas", "texas longhorns", "longhorns"], sport: Sport::Cfb },
    TeamEntry { abbrev: "USC", keywords: &["usc", "southern california", "trojans"], sport: Sport::Cfb },
    TeamEntry { abbrev: "LSU", keywords: &["lsu", "louisiana state", "lsu tigers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "CLEM", keywords: &["clemson", "clemson tigers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ORE", keywords: &["oregon", "oregon ducks", "ducks"], sport: Sport::Cfb },
    TeamEntry { abbrev: "PSU", keywords: &["penn state", "penn state nittany lions"], sport: Sport::Cfb },
    TeamEntry { abbrev: "FLA", keywords: &["florida", "florida gators", "gators"], sport: Sport::Cfb },
    TeamEntry { abbrev: "AUB", keywords: &["auburn", "auburn tigers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TENN", keywords: &["tennessee", "tennessee volunteers", "vols"], sport: Sport::Cfb },
    TeamEntry { abbrev: "OKLA", keywords: &["oklahoma", "oklahoma sooners", "sooners"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TXAM", keywords: &["texas a&m", "texas am", "aggies"], sport: Sport::Cfb },
    TeamEntry { abbrev: "NOTRD", keywords: &["notre dame", "notre dame fighting irish", "fighting irish"], sport: Sport::Cfb },
    TeamEntry { abbrev: "WASH", keywords: &["washington", "washington huskies"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MSU", keywords: &["michigan state", "michigan state spartans", "spartans"], sport: Sport::Cfb },
    TeamEntry { abbrev: "WIS", keywords: &["wisconsin", "wisconsin badgers", "badgers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "IOWA", keywords: &["iowa", "iowa hawkeyes", "hawkeyes"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ARK", keywords: &["arkansas", "arkansas razorbacks", "razorbacks"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MISS", keywords: &["ole miss", "mississippi", "rebels"], sport: Sport::Cfb },
    TeamEntry { abbrev: "COLO", keywords: &["colorado", "colorado buffaloes", "buffs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "BOISE", keywords: &["boise state", "boise state broncos"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UCF", keywords: &["ucf", "central florida", "knights"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MIA", keywords: &["miami", "miami hurricanes", "hurricanes"], sport: Sport::Cfb },
    TeamEntry { abbrev: "NCST", keywords: &["nc state", "north carolina state", "wolfpack"], sport: Sport::Cfb },
    TeamEntry { abbrev: "VT", keywords: &["virginia tech", "hokies"], sport: Sport::Cfb },
    TeamEntry { abbrev: "SC", keywords: &["south carolina", "gamecocks"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MIZZ", keywords: &["missouri", "mizzou", "missouri tigers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "KSU", keywords: &["kansas state", "kansas state wildcats"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TTU", keywords: &["texas tech", "red raiders"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TCU", keywords: &["tcu", "texas christian", "horned frogs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "BYU", keywords: &["byu", "brigham young", "cougars"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UTAH", keywords: &["utah", "utah utes", "utes"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ASU", keywords: &["arizona state", "sun devils"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ARIZ", keywords: &["arizona", "arizona wildcats"], sport: Sport::Cfb },
    TeamEntry { abbrev: "NEB", keywords: &["nebraska", "cornhuskers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ILL", keywords: &["illinois", "fighting illini", "illini"], sport: Sport::Cfb },
    TeamEntry { abbrev: "PUR", keywords: &["purdue", "boilermakers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "IU", keywords: &["indiana", "indiana hoosiers", "hoosiers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MINN", keywords: &["minnesota", "golden gophers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "STAN", keywords: &["stanford", "stanford cardinal"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UCLA", keywords: &["ucla", "ucla bruins"], sport: Sport::Cfb },
    TeamEntry { abbrev: "RUT", keywords: &["rutgers", "scarlet knights"], sport: Sport::Cfb },
    TeamEntry { abbrev: "NW", keywords: &["northwestern", "northwestern wildcats"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MARY", keywords: &["maryland", "maryland terrapins", "terps"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ISU", keywords: &["iowa state", "iowa state cyclones", "cyclones"], sport: Sport::Cfb },
    TeamEntry { abbrev: "KU", keywords: &["kansas", "kansas jayhawks", "jayhawks"], sport: Sport::Cfb },
    TeamEntry { abbrev: "CINCY", keywords: &["cincinnati", "cincinnati bearcats", "bearcats"], sport: Sport::Cfb },
    TeamEntry { abbrev: "HOU", keywords: &["houston", "houston cougars"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UCF", keywords: &["ucf", "central florida", "ucf knights"], sport: Sport::Cfb },
    TeamEntry { abbrev: "WVU", keywords: &["west virginia", "mountaineers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "PITT", keywords: &["pittsburgh", "pitt", "pitt panthers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "LOU", keywords: &["louisville", "louisville cardinals"], sport: Sport::Cfb },
    TeamEntry { abbrev: "WAKE", keywords: &["wake forest", "demon deacons"], sport: Sport::Cfb },
    TeamEntry { abbrev: "GT", keywords: &["georgia tech", "yellow jackets"], sport: Sport::Cfb },
    TeamEntry { abbrev: "SYR", keywords: &["syracuse", "syracuse orange"], sport: Sport::Cfb },
    TeamEntry { abbrev: "OKST", keywords: &["oklahoma state", "oklahoma state cowboys", "cowboys"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UK", keywords: &["kentucky", "kentucky wildcats"], sport: Sport::Cfb },
    TeamEntry { abbrev: "VANDY", keywords: &["vanderbilt", "vanderbilt commodores", "commodores"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MSST", keywords: &["mississippi state", "mississippi state bulldogs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "USF", keywords: &["south florida", "south florida bulls"], sport: Sport::Cfb },
    TeamEntry { abbrev: "BOISE", keywords: &["boise state", "boise state broncos", "broncos"], sport: Sport::Cfb },
    TeamEntry { abbrev: "SDSU", keywords: &["san diego state", "aztecs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "FRES", keywords: &["fresno state", "fresno state bulldogs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "SJSU", keywords: &["san jose state", "san jose state spartans"], sport: Sport::Cfb },
    TeamEntry { abbrev: "HAW", keywords: &["hawaii", "hawaii rainbow warriors"], sport: Sport::Cfb },
    TeamEntry { abbrev: "USU", keywords: &["utah state", "utah state aggies"], sport: Sport::Cfb },
    TeamEntry { abbrev: "AF", keywords: &["air force", "air force falcons", "falcons"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ARMY", keywords: &["army", "army black knights", "black knights"], sport: Sport::Cfb },
    TeamEntry { abbrev: "NAVY", keywords: &["navy", "navy midshipmen", "midshipmen"], sport: Sport::Cfb },
    TeamEntry { abbrev: "SMU", keywords: &["smu", "southern methodist", "mustangs"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TULN", keywords: &["tulane", "green wave"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MEMPH", keywords: &["memphis", "memphis tigers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "TLSA", keywords: &["tulsa", "tulsa golden hurricane"], sport: Sport::Cfb },
    TeamEntry { abbrev: "UNT", keywords: &["north texas", "mean green"], sport: Sport::Cfb },
    TeamEntry { abbrev: "WKU", keywords: &["western kentucky", "hilltoppers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "MARSH", keywords: &["marshall", "thundering herd"], sport: Sport::Cfb },
    TeamEntry { abbrev: "JMU", keywords: &["james madison", "james madison dukes", "dukes"], sport: Sport::Cfb },
    TeamEntry { abbrev: "APP", keywords: &["appalachian state", "app state", "mountaineers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "CCU", keywords: &["coastal carolina", "chanticleers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "LIB", keywords: &["liberty", "liberty flames", "flames"], sport: Sport::Cfb },
    TeamEntry { abbrev: "JAX", keywords: &["jacksonville state", "jacksonville state gamecocks"], sport: Sport::Cfb },
    TeamEntry { abbrev: "NMST", keywords: &["new mexico state", "new mexico state aggies"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ULM", keywords: &["louisiana monroe", "ul monroe", "warhawks"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ULL", keywords: &["louisiana lafayette", "ul lafayette", "ragin cajuns", "louisiana ragin' cajuns"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ARST", keywords: &["arkansas state", "arkansas state red wolves", "red wolves"], sport: Sport::Cfb },
    TeamEntry { abbrev: "GASO", keywords: &["georgia southern", "georgia southern eagles"], sport: Sport::Cfb },
    TeamEntry { abbrev: "GAST", keywords: &["georgia state", "georgia state panthers"], sport: Sport::Cfb },
    TeamEntry { abbrev: "ODU", keywords: &["old dominion", "old dominion monarchs", "monarchs"], sport: Sport::Cfb },
    // ── PGA (top golfers) ───────────────────────────────────────────
    TeamEntry { abbrev: "SCHEFFLER", keywords: &["scottie scheffler", "scheffler"], sport: Sport::Pga },
    TeamEntry { abbrev: "MCILROY", keywords: &["rory mcilroy", "mcilroy", "rory"], sport: Sport::Pga },
    TeamEntry { abbrev: "RAHM", keywords: &["jon rahm", "rahm"], sport: Sport::Pga },
    TeamEntry { abbrev: "KOEPKA", keywords: &["brooks koepka", "koepka"], sport: Sport::Pga },
    TeamEntry { abbrev: "THOMAS", keywords: &["justin thomas", "jt"], sport: Sport::Pga },
    TeamEntry { abbrev: "SPIETH", keywords: &["jordan spieth", "spieth"], sport: Sport::Pga },
    TeamEntry { abbrev: "CANTLAY", keywords: &["patrick cantlay", "cantlay"], sport: Sport::Pga },
    TeamEntry { abbrev: "HOVLAND", keywords: &["viktor hovland", "hovland"], sport: Sport::Pga },
    TeamEntry { abbrev: "CLARK", keywords: &["wyndham clark", "clark"], sport: Sport::Pga },
    TeamEntry { abbrev: "MORIKAWA", keywords: &["collin morikawa", "morikawa"], sport: Sport::Pga },
    TeamEntry { abbrev: "SCHAUFFELE", keywords: &["xander schauffele", "schauffele", "xander"], sport: Sport::Pga },
    TeamEntry { abbrev: "DECHAMBEAU", keywords: &["bryson dechambeau", "dechambeau", "bryson"], sport: Sport::Pga },
    TeamEntry { abbrev: "FINAU", keywords: &["tony finau", "finau"], sport: Sport::Pga },
    TeamEntry { abbrev: "MATSUYAMA", keywords: &["hideki matsuyama", "matsuyama"], sport: Sport::Pga },
    TeamEntry { abbrev: "FLEETWOOD", keywords: &["tommy fleetwood", "fleetwood"], sport: Sport::Pga },
    TeamEntry { abbrev: "BURNS", keywords: &["sam burns", "burns"], sport: Sport::Pga },
    TeamEntry { abbrev: "HARMAN", keywords: &["brian harman", "harman"], sport: Sport::Pga },
    TeamEntry { abbrev: "HOMA", keywords: &["max homa", "homa"], sport: Sport::Pga },
    TeamEntry { abbrev: "ABERG", keywords: &["ludvig aberg", "aberg", "ludvig åberg"], sport: Sport::Pga },
    TeamEntry { abbrev: "CONNERS", keywords: &["corey conners", "conners"], sport: Sport::Pga },
    TeamEntry { abbrev: "WOODLAND", keywords: &["gary woodland", "woodland"], sport: Sport::Pga },
    TeamEntry { abbrev: "FITZPATRICK", keywords: &["matt fitzpatrick", "fitzpatrick"], sport: Sport::Pga },
    TeamEntry { abbrev: "DJTOUR", keywords: &["dustin johnson", "dj"], sport: Sport::Pga },
    TeamEntry { abbrev: "REED", keywords: &["patrick reed", "reed"], sport: Sport::Pga },
    TeamEntry { abbrev: "SUNGJAE", keywords: &["sungjae im", "im sungjae", "im"], sport: Sport::Pga },
    TeamEntry { abbrev: "KIM", keywords: &["si woo kim", "kim si woo"], sport: Sport::Pga },
    TeamEntry { abbrev: "ROSE", keywords: &["justin rose", "rose"], sport: Sport::Pga },
    TeamEntry { abbrev: "LOWRY", keywords: &["shane lowry", "lowry"], sport: Sport::Pga },
    TeamEntry { abbrev: "FOWLER", keywords: &["rickie fowler", "fowler"], sport: Sport::Pga },
    TeamEntry { abbrev: "BRADLEY", keywords: &["keegan bradley", "bradley"], sport: Sport::Pga },
    // ── Tennis (top ATP/WTA players) ────────────────────────────────
    TeamEntry { abbrev: "SINNER", keywords: &["jannik sinner", "sinner"], sport: Sport::Tennis },
    TeamEntry { abbrev: "DJOKOVIC", keywords: &["novak djokovic", "djokovic", "nole", "novak"], sport: Sport::Tennis },
    TeamEntry { abbrev: "ALCARAZ", keywords: &["carlos alcaraz", "alcaraz", "carlitos"], sport: Sport::Tennis },
    TeamEntry { abbrev: "MEDVEDEV", keywords: &["daniil medvedev", "medvedev"], sport: Sport::Tennis },
    TeamEntry { abbrev: "ZVEREV", keywords: &["alexander zverev", "zverev", "sascha"], sport: Sport::Tennis },
    TeamEntry { abbrev: "RUBLEV", keywords: &["andrey rublev", "rublev"], sport: Sport::Tennis },
    TeamEntry { abbrev: "RUUD", keywords: &["casper ruud", "ruud"], sport: Sport::Tennis },
    TeamEntry { abbrev: "HURKACZ", keywords: &["hubert hurkacz", "hurkacz"], sport: Sport::Tennis },
    TeamEntry { abbrev: "FRITZ", keywords: &["taylor fritz", "fritz"], sport: Sport::Tennis },
    TeamEntry { abbrev: "TSITSIPAS", keywords: &["stefanos tsitsipas", "tsitsipas", "stef"], sport: Sport::Tennis },
    TeamEntry { abbrev: "DEMIN", keywords: &["karen khachanov", "khachanov"], sport: Sport::Tennis },
    TeamEntry { abbrev: "TIAFOE", keywords: &["frances tiafoe", "tiafoe", "big foe"], sport: Sport::Tennis },
    TeamEntry { abbrev: "PAUL", keywords: &["tommy paul", "tommy paul tennis"], sport: Sport::Tennis },
    TeamEntry { abbrev: "SHELTON", keywords: &["ben shelton", "shelton"], sport: Sport::Tennis },
    TeamEntry { abbrev: "DRAPER", keywords: &["jack draper", "draper"], sport: Sport::Tennis },
    TeamEntry { abbrev: "MUSETTI", keywords: &["lorenzo musetti", "musetti"], sport: Sport::Tennis },
    TeamEntry { abbrev: "AUGER", keywords: &["felix auger aliassime", "auger-aliassime", "faa"], sport: Sport::Tennis },
    TeamEntry { abbrev: "DIMITROV", keywords: &["grigor dimitrov", "dimitrov"], sport: Sport::Tennis },
    TeamEntry { abbrev: "JARRY", keywords: &["nicolas jarry", "jarry"], sport: Sport::Tennis },
    TeamEntry { abbrev: "BERRETTINI", keywords: &["matteo berrettini", "berrettini"], sport: Sport::Tennis },
    TeamEntry { abbrev: "CERUNDOLO", keywords: &["francisco cerundolo", "cerundolo"], sport: Sport::Tennis },
    TeamEntry { abbrev: "BUBLIK", keywords: &["alexander bublik", "bublik"], sport: Sport::Tennis },
    TeamEntry { abbrev: "NORRIE", keywords: &["cameron norrie", "norrie"], sport: Sport::Tennis },
    TeamEntry { abbrev: "BAEZ", keywords: &["sebastian baez", "baez"], sport: Sport::Tennis },
    TeamEntry { abbrev: "THOMPSON", keywords: &["jordan thompson", "thompson"], sport: Sport::Tennis },
    // WTA top players
    TeamEntry { abbrev: "SWIATEK", keywords: &["iga swiatek", "swiatek", "iga świątek"], sport: Sport::Tennis },
    TeamEntry { abbrev: "SABALENKA", keywords: &["aryna sabalenka", "sabalenka"], sport: Sport::Tennis },
    TeamEntry { abbrev: "GAUFF", keywords: &["coco gauff", "gauff", "coco"], sport: Sport::Tennis },
    TeamEntry { abbrev: "RYBAKINA", keywords: &["elena rybakina", "rybakina"], sport: Sport::Tennis },
    TeamEntry { abbrev: "PEGULA", keywords: &["jessica pegula", "pegula"], sport: Sport::Tennis },
    TeamEntry { abbrev: "ZHENG", keywords: &["zheng qinwen", "qinwen zheng"], sport: Sport::Tennis },
    TeamEntry { abbrev: "JABEUR", keywords: &["ons jabeur", "jabeur"], sport: Sport::Tennis },
    TeamEntry { abbrev: "KEYS", keywords: &["madison keys", "keys"], sport: Sport::Tennis },
    TeamEntry { abbrev: "VONDROU", keywords: &["marketa vondrousova", "vondrousova"], sport: Sport::Tennis },
    TeamEntry { abbrev: "KREJA", keywords: &["barbora krejcikova", "krejcikova"], sport: Sport::Tennis },
    TeamEntry { abbrev: "KOSTYUK", keywords: &["marta kostyuk", "kostyuk"], sport: Sport::Tennis },
    TeamEntry { abbrev: "AZARENKA", keywords: &["victoria azarenka", "azarenka", "vika"], sport: Sport::Tennis },
    TeamEntry { abbrev: "OSTAPENKO", keywords: &["jelena ostapenko", "ostapenko"], sport: Sport::Tennis },
    TeamEntry { abbrev: "BENCIC", keywords: &["belinda bencic", "bencic"], sport: Sport::Tennis },
    TeamEntry { abbrev: "SAMSONOVA", keywords: &["liudmila samsonova", "samsonova"], sport: Sport::Tennis },
    TeamEntry { abbrev: "ANDREEVA", keywords: &["mirra andreeva", "andreeva"], sport: Sport::Tennis },
    TeamEntry { abbrev: "NAVARRO", keywords: &["emma navarro", "navarro"], sport: Sport::Tennis },
    TeamEntry { abbrev: "MUCHOVA", keywords: &["karolina muchova", "muchova"], sport: Sport::Tennis },
    TeamEntry { abbrev: "HADDAD", keywords: &["beatriz haddad maia", "haddad maia"], sport: Sport::Tennis },
    TeamEntry { abbrev: "PAOLINI", keywords: &["jasmine paolini", "paolini"], sport: Sport::Tennis },
];

/// Pre-built lookup: lowercase keyword → abbreviation.
/// For sport-scoped lookups, call `lookup_team`.
static KEYWORD_TO_ABBREV: LazyLock<HashMap<(&'static str, Sport), &'static str>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        for entry in TEAM_ENTRIES {
            for &kw in entry.keywords {
                map.insert((kw, entry.sport), entry.abbrev);
            }
        }
        map
    });

/// Flat keyword → abbreviation map (ignoring sport scope).
/// Used when the sport context is already known from bucketing.
static FLAT_KEYWORD_TO_ABBREV: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        for entry in TEAM_ENTRIES {
            for &kw in entry.keywords {
                map.entry(kw).or_insert(entry.abbrev);
            }
        }
        map
    });

/// Resolve a team name (full or partial) to its canonical abbreviation.
///
/// Tries (in order):
/// 1. Exact match in the sport-scoped keyword table
/// 2. Exact match in the flat keyword table (cross-sport)
/// 3. Substring containment (if any keyword is contained in the input)
/// 4. Direct abbreviation match (input is already an abbreviation)
pub fn lookup_team(name: &str, sport: Option<Sport>) -> Option<String> {
    let lower = name.to_lowercase();
    let trimmed = lower.trim();

    // 1. Sport-scoped exact match
    if let Some(sport) = sport {
        if let Some(&abbrev) = KEYWORD_TO_ABBREV.get(&(trimmed, sport)) {
            return Some(abbrev.to_string());
        }
        // Also try with sport-scoped keyword containment
        for entry in TEAM_ENTRIES {
            if entry.sport != sport {
                continue;
            }
            for &kw in entry.keywords {
                if trimmed.contains(kw) {
                    return Some(entry.abbrev.to_string());
                }
            }
        }
    }

    // 2. Flat exact match
    if let Some(&abbrev) = FLAT_KEYWORD_TO_ABBREV.get(trimmed) {
        return Some(abbrev.to_string());
    }

    // 3. Substring containment (longest keyword first for precision)
    let mut best: Option<(&str, usize)> = None;
    for entry in TEAM_ENTRIES {
        for &kw in entry.keywords {
            if trimmed.contains(kw) && kw.len() > best.map_or(0, |(_, l)| l) {
                best = Some((entry.abbrev, kw.len()));
            }
        }
    }
    if let Some((abbrev, _)) = best {
        return Some(abbrev.to_string());
    }

    // 4. Input is already a known abbreviation (uppercase check)
    let upper = name.to_uppercase();
    for entry in TEAM_ENTRIES {
        if entry.abbrev == upper {
            return Some(upper);
        }
    }

    None
}

/// Like `lookup_team`, but if no match is found and the input looks like a
/// short alphabetic abbreviation (2-6 chars), pass it through unchanged.
/// Used specifically for Kalshi ticker-derived team abbreviations where
/// an unknown abbrev is still useful as-is.
pub fn lookup_team_or_passthrough(name: &str, sport: Option<Sport>) -> String {
    if let Some(resolved) = lookup_team(name, sport) {
        return resolved;
    }
    let trimmed = name.trim();
    let upper = trimmed.to_uppercase();
    if (2..=6).contains(&upper.len()) && upper.chars().all(|c| c.is_ascii_alphabetic()) {
        return upper;
    }
    trimmed.to_string()
}

/// Resolve a team name to its canonical abbreviation, constrained by
/// a set of valid abbreviations (e.g., Kalshi outcome names).
/// This is the replacement for `match_gamma_label_to_kalshi` in dome_poller.
pub fn match_label_to_abbrev(label: &str, valid_abbrevs: &[String]) -> Option<String> {
    let lower = label.to_lowercase();

    // Try the keyword table
    for entry in TEAM_ENTRIES {
        for &kw in entry.keywords {
            if lower.contains(kw) && valid_abbrevs.iter().any(|a| a == entry.abbrev) {
                return Some(entry.abbrev.to_string());
            }
        }
    }

    // Direct abbreviation match
    let upper = label.to_uppercase();
    if valid_abbrevs.iter().any(|a| a == &upper) {
        return Some(upper);
    }

    // Containment: any valid abbreviation is a substring of the label
    for abbrev in valid_abbrevs {
        if upper.contains(abbrev.as_str()) {
            return Some(abbrev.clone());
        }
    }

    // Prefix: label starts with an abbreviation
    for abbrev in valid_abbrevs {
        if lower.starts_with(&abbrev.to_lowercase()) {
            return Some(abbrev.clone());
        }
    }

    None
}

/// Fuzzy score: how well `abbr` characters form an ordered subsequence in `full_name`.
/// Both inputs should be uppercase. Extracted from dome_poller for shared use.
pub fn subsequence_score(full_name: &str, abbr: &str) -> i32 {
    let mut score = 0;
    let mut full_chars = full_name.chars();
    for a_char in abbr.chars() {
        if !a_char.is_alphanumeric() {
            continue;
        }
        while let Some(f_char) = full_chars.next() {
            if f_char == a_char {
                score += 1;
                break;
            }
        }
    }
    score
}

/// Kalshi series ticker → Sport mapping.
#[allow(dead_code)]
pub fn series_ticker_to_sport(series: &str) -> Option<Sport> {
    let upper = series.to_uppercase();
    if upper.starts_with("KXNFL") { return Some(Sport::Nfl); }
    if upper.starts_with("KXNBA") { return Some(Sport::Nba); }
    if upper.starts_with("KXMLB") { return Some(Sport::Mlb); }
    if upper.starts_with("KXNHL") { return Some(Sport::Nhl); }
    if upper.starts_with("KXCFB") { return Some(Sport::Cfb); }
    if upper.starts_with("KXCBB") { return Some(Sport::Cbb); }
    if upper.starts_with("KXPGA") { return Some(Sport::Pga); }
    if upper.starts_with("KXTEN") { return Some(Sport::Tennis); }
    None
}

/// Sport → Kalshi series ticker prefixes (used for fetching).
pub fn sport_to_kalshi_series(sport: Sport) -> &'static [&'static str] {
    match sport {
        Sport::Nfl => &["KXNFLGAME"],
        Sport::Nba => &["KXNBAGAME"],
        Sport::Mlb => &["KXMLBGAME"],
        Sport::Nhl => &["KXNHLGAME"],
        Sport::Cfb => &["KXCFBGAME"],
        Sport::Cbb => &["KXCBBGAME"],
        Sport::Pga => &["KXPGAGAME", "KXPGA"],
        Sport::Tennis => &["KXTENGAME", "KXTEN"],
        _ => &[],
    }
}

/// Sport → Polymarket Gamma API tag strings.
pub fn sport_to_polymarket_tags(sport: Sport) -> &'static [&'static str] {
    match sport {
        Sport::Nfl => &["nfl", "football", "Games"],
        Sport::Nba => &["nba", "basketball", "Games"],
        Sport::Mlb => &["mlb", "baseball", "Games"],
        Sport::Nhl => &["nhl", "hockey", "Games"],
        Sport::Cfb => &["ncaaf", "college football"],
        Sport::Cbb => &["ncaab", "college basketball"],
        Sport::Pga => &["pga", "golf"],
        Sport::Tennis => &["tennis", "atp", "wta"],
        _ => &[],
    }
}

/// Extract outcome abbreviation from a Kalshi market ticker.
/// e.g., "KXNFLGAME-25AUG16ARIDEN-ARI" → "ARI"
pub fn extract_kalshi_outcome(ticker: &str) -> String {
    ticker
        .rsplit('-')
        .next()
        .unwrap_or("UNKNOWN")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_nba_team_by_mascot() {
        assert_eq!(lookup_team("lakers", Some(Sport::Nba)).as_deref(), Some("LAL"));
        assert_eq!(lookup_team("celtics", Some(Sport::Nba)).as_deref(), Some("BOS"));
        assert_eq!(lookup_team("warriors", Some(Sport::Nba)).as_deref(), Some("GSW"));
    }

    #[test]
    fn test_lookup_nba_team_by_city() {
        assert_eq!(lookup_team("golden state", Some(Sport::Nba)).as_deref(), Some("GSW"));
        assert_eq!(lookup_team("san antonio", Some(Sport::Nba)).as_deref(), Some("SAS"));
    }

    #[test]
    fn test_lookup_nba_team_full_name() {
        assert_eq!(lookup_team("los angeles lakers", Some(Sport::Nba)).as_deref(), Some("LAL"));
        assert_eq!(lookup_team("Boston Celtics", Some(Sport::Nba)).as_deref(), Some("BOS"));
    }

    #[test]
    fn test_lookup_nfl_team() {
        assert_eq!(lookup_team("chiefs", Some(Sport::Nfl)).as_deref(), Some("KC"));
        assert_eq!(lookup_team("49ers", Some(Sport::Nfl)).as_deref(), Some("SF"));
        assert_eq!(lookup_team("packers", Some(Sport::Nfl)).as_deref(), Some("GB"));
    }

    #[test]
    fn test_lookup_mlb_team() {
        assert_eq!(lookup_team("yankees", Some(Sport::Mlb)).as_deref(), Some("NYY"));
        assert_eq!(lookup_team("dodgers", Some(Sport::Mlb)).as_deref(), Some("LAD"));
    }

    #[test]
    fn test_lookup_nhl_team() {
        assert_eq!(lookup_team("golden knights", Some(Sport::Nhl)).as_deref(), Some("VGK"));
        assert_eq!(lookup_team("maple leafs", Some(Sport::Nhl)).as_deref(), Some("TOR"));
    }

    #[test]
    fn test_lookup_abbreviation_directly() {
        assert_eq!(lookup_team("LAL", None).as_deref(), Some("LAL"));
        assert_eq!(lookup_team("GSW", None).as_deref(), Some("GSW"));
    }

    #[test]
    fn test_match_label_to_abbrev() {
        let valid = vec!["LAL".to_string(), "BOS".to_string()];
        assert_eq!(match_label_to_abbrev("Lakers", &valid).as_deref(), Some("LAL"));
        assert_eq!(match_label_to_abbrev("Boston Celtics", &valid).as_deref(), Some("BOS"));
    }

    #[test]
    fn test_extract_kalshi_outcome() {
        assert_eq!(extract_kalshi_outcome("KXNFLGAME-25AUG16ARIDEN-ARI"), "ARI");
        assert_eq!(extract_kalshi_outcome("KXNBAGAME-26MAR14LALBOS-LAL"), "LAL");
    }

    #[test]
    fn test_subsequence_score() {
        assert!(subsequence_score("LAKERS", "LAL") > 0);
        assert!(subsequence_score("CELTICS", "BOS") == 0);
        assert_eq!(subsequence_score("LAKERS", "LAK"), 3);
    }

    // ── CBB team lookups ────────────────────────────────────────
    #[test]
    fn test_lookup_cbb_duke() {
        assert_eq!(lookup_team("duke", Some(Sport::Cbb)).as_deref(), Some("DUK"));
        assert_eq!(lookup_team("blue devils", Some(Sport::Cbb)).as_deref(), Some("DUK"));
        assert_eq!(lookup_team("Duke Blue Devils", Some(Sport::Cbb)).as_deref(), Some("DUK"));
    }

    #[test]
    fn test_lookup_cbb_unc() {
        assert_eq!(lookup_team("north carolina", Some(Sport::Cbb)).as_deref(), Some("UNC"));
        assert_eq!(lookup_team("tar heels", Some(Sport::Cbb)).as_deref(), Some("UNC"));
        assert_eq!(lookup_team("unc", Some(Sport::Cbb)).as_deref(), Some("UNC"));
    }

    #[test]
    fn test_lookup_cbb_gonzaga() {
        assert_eq!(lookup_team("gonzaga", Some(Sport::Cbb)).as_deref(), Some("GONZ"));
        assert_eq!(lookup_team("zags", Some(Sport::Cbb)).as_deref(), Some("GONZ"));
    }

    #[test]
    fn test_lookup_cbb_uconn() {
        assert_eq!(lookup_team("uconn", Some(Sport::Cbb)).as_deref(), Some("CONN"));
        assert_eq!(lookup_team("connecticut huskies", Some(Sport::Cbb)).as_deref(), Some("CONN"));
    }

    #[test]
    fn test_lookup_nhl_utah() {
        assert_eq!(lookup_team("utah hockey club", Some(Sport::Nhl)).as_deref(), Some("UTA"));
    }

    #[test]
    fn test_lookup_cfb_notre_dame() {
        assert_eq!(lookup_team("notre dame", Some(Sport::Cfb)).as_deref(), Some("NOTRD"));
        assert_eq!(lookup_team("fighting irish", Some(Sport::Cfb)).as_deref(), Some("NOTRD"));
    }

    #[test]
    fn test_sport_to_kalshi_series_cbb() {
        let series = sport_to_kalshi_series(Sport::Cbb);
        assert!(series.contains(&"KXCBBGAME"));
    }

    #[test]
    fn test_sport_to_polymarket_tags_cbb() {
        let tags = sport_to_polymarket_tags(Sport::Cbb);
        assert!(tags.contains(&"ncaab") || tags.contains(&"college basketball"));
    }

    #[test]
    fn test_city_disambiguation_with_sport_scope() {
        // "chicago" should resolve to CHI in NBA (Bulls) context
        assert_eq!(lookup_team("chicago", Some(Sport::Nba)).as_deref(), Some("CHI"));
        // "houston" should resolve to HOU in NBA (Rockets) context
        assert_eq!(lookup_team("houston", Some(Sport::Nba)).as_deref(), Some("HOU"));
        // "denver" should resolve to DEN in NBA (Nuggets) context
        assert_eq!(lookup_team("denver", Some(Sport::Nba)).as_deref(), Some("DEN"));
    }

    #[test]
    fn test_case_insensitive_lookup() {
        assert_eq!(lookup_team("LAKERS", Some(Sport::Nba)).as_deref(), Some("LAL"));
        assert_eq!(lookup_team("Lakers", Some(Sport::Nba)).as_deref(), Some("LAL"));
        assert_eq!(lookup_team("lakers", Some(Sport::Nba)).as_deref(), Some("LAL"));
    }

    #[test]
    fn test_match_label_to_abbrev_nhl() {
        let valid = vec!["TOR".to_string(), "BOS".to_string()];
        assert_eq!(match_label_to_abbrev("Maple Leafs", &valid).as_deref(), Some("TOR"));
        assert_eq!(match_label_to_abbrev("Bruins", &valid).as_deref(), Some("BOS"));
    }

    #[test]
    fn test_match_label_to_abbrev_cbb() {
        let valid = vec!["DUK".to_string(), "UNC".to_string()];
        assert_eq!(match_label_to_abbrev("Duke", &valid).as_deref(), Some("DUK"));
        assert_eq!(match_label_to_abbrev("North Carolina", &valid).as_deref(), Some("UNC"));
    }

    #[test]
    fn test_kalshi_abbreviation_variants_nhl() {
        assert_eq!(lookup_team("NJ", Some(Sport::Nhl)).as_deref(), Some("NJD"));
        assert_eq!(lookup_team("SJ", Some(Sport::Nhl)).as_deref(), Some("SJS"));
        assert_eq!(lookup_team("WAS", Some(Sport::Nhl)).as_deref(), Some("WSH"));
        assert_eq!(lookup_team("TBL", Some(Sport::Nhl)).as_deref(), Some("TB"));
        assert_eq!(lookup_team("VGS", Some(Sport::Nhl)).as_deref(), Some("VGK"));
        assert_eq!(lookup_team("LAK", Some(Sport::Nhl)).as_deref(), Some("LAK"));
    }

    #[test]
    fn test_kalshi_abbreviation_variants_nba() {
        assert_eq!(lookup_team("GS", Some(Sport::Nba)).as_deref(), Some("GSW"));
        assert_eq!(lookup_team("NO", Some(Sport::Nba)).as_deref(), Some("NOP"));
        assert_eq!(lookup_team("BK", Some(Sport::Nba)).as_deref(), Some("BKN"));
        assert_eq!(lookup_team("PHO", Some(Sport::Nba)).as_deref(), Some("PHX"));
        assert_eq!(lookup_team("SA", Some(Sport::Nba)).as_deref(), Some("SAS"));
    }

    #[test]
    fn test_kalshi_abbreviation_variants_nfl() {
        assert_eq!(lookup_team("JAC", Some(Sport::Nfl)).as_deref(), Some("JAX"));
    }

    #[test]
    fn test_kalshi_abbreviation_variants_mlb() {
        assert_eq!(lookup_team("CWS", Some(Sport::Mlb)).as_deref(), Some("CHW"));
    }
}
