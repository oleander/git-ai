
#[derive(StructOpt)]
enum Command {
    /// Pound acorns into flour for cookie dough.
    Pound {
        acorns: u32,
    },

    /// Add magical sparkles -- the secret ingredient!
    Sparkle {
        #[structopt(short, parse(from_occurrences))]
        magicality: u64,
        #[structopt(short)]
        color: String,
    },

    Finish(Finish),
}

// Subcommand can also be externalized by using a 1-uple enum variant
#[derive(StructOpt)]
struct Finish {
   #[structopt(short)]
   time: u32,

   #[structopt(subcommand)] // Note that we mark a field as a subcommand
   finish_type: FinishType,
}

// subsubcommand!
#[derive(StructOpt)]
enum FinishType {
   Glaze {
       applications: u32,
   },

   Powder {
       flavor: String,
       dips: u32,
   }
}

#[derive(StructOpt)]
struct Foo {
   file: String,

   #[structopt(subcommand)]
   cmd: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
   Bar,
   Baz,
   Quux,
}

#[derive(Debug, PartialEq, StructOpt)]
struct Opt {
   #[structopt(subcommand)]
   sub: Subcommands,
}

#[derive(Debug, PartialEq, StructOpt)]
enum Subcommands {
   // normal subcommand
   Add,

   // `external_subcommand` tells structopt to put all the extra arguments into this Vec
   #[structopt(external_subcommand)]
   Other(Vec<String>),
}

// normal subcommand
assert_eq!(
   Opt::from_iter(&["test", "add"]),
   Opt {
       sub: Subcommands::Add
   }
);

assert_eq!(
   Opt::from_iter(&["test", "git", "status"]),
   Opt {
       sub: Subcommands::Other(vec!["git".into(), "status".into()])
   }
);

assert!(Opt::from_iter_safe(&["test"]).is_err());

#[derive(StructOpt)]
enum BaseCli {
   Ghost10 {
       arg1: i32,
   }
}

#[derive(StructOpt)]
enum Opt {
   #[structopt(flatten)]
   BaseCli(BaseCli),

   Dex {
       arg2: i32,
   },
}

#[derive(StructOpt)]
enum Command {
   Bar,
   Baz,
   Quux,
}
```
