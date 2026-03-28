use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

pub type PgConnectionPool = Pool<ConnectionManager<PgConnection>>;

pub fn connect(database_url: &str) -> PgConnectionPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Could not build connection pool")
}
