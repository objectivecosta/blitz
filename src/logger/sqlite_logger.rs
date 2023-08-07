use sqlite::State;

pub trait Logger {
  fn log_traffic(&self, from_ip: &str, from_dns: &str, to_ip: &str, to_dns: &str, packet_size: i64, payload_size: i64) -> bool;
}

pub struct SQLiteLogger {
  connection: sqlite::Connection
}

impl SQLiteLogger {
  pub fn new(path: &str) -> Self {
    Self {
      connection: sqlite::open(path).unwrap(),
    }
  }

  pub fn migrate(&self) {
    let query = "
    CREATE TABLE traffic (from_ip TEXT, from_dns TEXT, to_ip TEXT, to_dns TEXT, packet_size INTEGER, payload_size  INTEGER);
    ";

    println!("Logger MIGRATE!");

    self.connection.execute(query).unwrap();
  }
}

impl Logger for SQLiteLogger {
    fn log_traffic(&self, from_ip: &str, from_dns: &str, to_ip: &str, to_dns: &str, packet_size: i64, payload_size: i64) -> bool {

      println!("[log_traffic] {} ({}) -> {} ({}). Sizes: {} ({})", from_ip, from_dns, to_ip, to_dns, packet_size, payload_size);

      let query = "
      INSERT INTO traffic VALUES (?, ?, ?, ?, ?, ?);
      ";

      let mut statement = self.connection.prepare(query).unwrap();
      statement.bind((1, to_ip)).unwrap();
      statement.bind((2, to_dns)).unwrap();

      statement.bind((3, from_ip)).unwrap();
      statement.bind((4, from_dns)).unwrap();

      statement.bind((5, packet_size)).unwrap();
      statement.bind((6, payload_size)).unwrap();

      let result = statement.next();

      if let Ok(State::Done) = result {
        return true;
      }

      return false;
    }
}