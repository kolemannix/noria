extern crate distributary;

use distributary::ControllerBuilder;

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    // inline recipe definition
    let sql = "# base tables
               CREATE TABLE Article (aid int, title varchar(255), \
                                     url text, PRIMARY KEY(aid));
               CREATE TABLE Vote (aid int, uid int);

               # read queries
               VoteCount: SELECT Vote.aid, COUNT(uid) AS votes \
                            FROM Vote GROUP BY Vote.aid;
               QUERY ArticleWithVoteCount: \
                            SELECT Article.aid, title, url, VoteCount.votes AS votes \
                            FROM Article, VoteCount \
                            WHERE Article.aid = VoteCount.aid AND Article.aid = ?;";

    let persistence_params = distributary::PersistenceParameters::new(
        distributary::DurabilityMode::Permanent,
        512,
        Duration::from_millis(1),
        Some(String::from("evict-o-rama")),
    );

    // set up Soup via recipe
    let mut builder = ControllerBuilder::default();
    builder.log_with(distributary::logger_pls());
    builder.set_worker_threads(2);
    builder.set_persistence(persistence_params);
    builder.set_memory_limit(1024 * 1024);

    // TODO: This should be removed when the `it_works_with_reads_before_writes`
    // test passes again.
    builder.disable_partial();

    let mut blender = builder.build_local();
    blender.install_recipe(sql.to_owned()).unwrap();

    // Get mutators and getter.
    let mut article = blender.get_mutator("Article").unwrap();
    let mut vote = blender.get_mutator("Vote").unwrap();
    let mut awvc = blender.get_getter("ArticleWithVoteCount").unwrap();

    println!("Creating articles...");
    for aid in 1..10_000 {
        // Make sure the article exists:
        if awvc.lookup(&aid.into(), true).unwrap().is_empty() {
            let title = format!("Article {}", aid);
            let url = "http://pdos.csail.mit.edu";
            article
                .put(vec![aid.into(), title.into(), url.into()])
                .unwrap();
        }
    }

    // Then create a new vote:
    println!("Casting votes...");
    loop {
        let uid = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let aid = uid % 10_000;
        vote.put(vec![aid.into(), uid.into()]).unwrap();
    }
}
