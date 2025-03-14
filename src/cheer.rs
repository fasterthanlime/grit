use color_eyre::owo_colors::OwoColorize;

pub(crate) fn cheer() {
    let random_emoji = MARINE_EMOJIS[fastrand::usize(..MARINE_EMOJIS.len())];

    let message1 = CHEERFUL_MESSAGES[fastrand::usize(..CHEERFUL_MESSAGES.len())];
    let message2 = CHEERFUL_MESSAGES[fastrand::usize(..CHEERFUL_MESSAGES.len())];

    eprintln!("{}", "========================================".cyan());
    eprintln!("{} {}", random_emoji, message1.green().bold());
    eprintln!("{} {}", random_emoji, message2.blue());
    eprintln!("{}", "========================================".cyan());
}

const MARINE_EMOJIS: [&str; 10] = ["🐠", "🐡", "🦈", "🐙", "🦀", "🐚", "🐳", "🐬", "🦭", "🐟"];

const CHEERFUL_MESSAGES: [&str; 50] = [
    "Everything's shipshape and Bristol fashion!",
    "You're on top of your game!",
    "Smooth sailing ahead!",
    "You're crushing it!",
    "High five for being up-to-date!",
    "You're a git wizard extraordinaire!",
    "Code so fresh, it should be illegal!",
    "Repo goals achieved!",
    "You're in sync with the universe!",
    "Git-tastic work!",
    "You've got your ducks in a row!",
    "Cleaner than a whistle!",
    "Your repo game is strong!",
    "Synced and ready to rock!",
    "You're at the helm of this ship!",
    "Smooth as butter!",
    "Git-er done? More like git-er already done!",
    "You're firing on all cylinders!",
    "Repo perfection achieved!",
    "You're in the git zone!",
    "Commits so clean, they sparkle!",
    "Your repo is a thing of beauty!",
    "Git-standing work!",
    "You're a syncing machine!",
    "Repository bliss achieved!",
    "You're a git expert!",
    "Synced to perfection!",
    "Your repo is a work of art!",
    "Git-cellent job!",
    "You're on fire (in a good way)!",
    "Repo harmony restored!",
    "You've got the golden touch!",
    "Git-tacular performance!",
    "You're a git whisperer!",
    "Synced and fabulous!",
    "Your repo is a shining example!",
    "Git-credible work!",
    "You're in perfect harmony!",
    "Repo nirvana achieved!",
    "You're a git superhero!",
    "Synced to the nines!",
    "Your repo is poetry in motion!",
    "Git-mazing job!",
    "You're a champion of version control!",
    "Repo zen achieved!",
    "You've got git-game!",
    "Synced and sensational!",
    "Your repo is a masterpiece!",
    "Git-errific work!",
    "You're a git guru!",
];
