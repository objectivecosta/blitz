use std::{cell::RefCell, time::SystemTime};

use chrono::{DateTime, Utc};
use rusqlite::{params, OpenFlags};

pub trait Logger {
    fn log_traffic(
        &mut self,
        timestamp: i64,
        from_ip: &str,
        from_dns: &str,
        to_ip: &str,
        to_dns: &str,
        packet_size: i64,
        payload_size: i64,
    ) -> bool;
}

pub struct SQLiteLogger {
    connection: rusqlite::Connection,
    last_today: RefCell<String>,
}

impl SQLiteLogger {
    pub fn new(path: &str) -> Self {
        Self {
            connection: rusqlite::Connection::open_with_flags(
                path, 
                OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
            ).unwrap(),
            last_today: RefCell::from("".to_owned()),
        }
    }

    pub fn setup_table(&self) {
        if !self.contains_today() {
            self.create_today_table();
        }
    }

    fn today(&self) -> String {
        let now = SystemTime::now();
        let date_time: DateTime<Utc> = chrono::DateTime::from(now);
        let date_time_format = "%Y%m%d";
        let formatted = date_time.format(date_time_format).to_string();
        return formatted;
    }

    fn today_table(&self) -> String {
        return format!("traffic_{}", self.today());
    }

    fn contains_today(&self) -> bool {
        let query = format!(
            "SELECT count(*) as total FROM sqlite_master WHERE type= 'table' AND name = ?;"
        );

        let mut statement = self.connection.prepare(&query).unwrap();

        let mut last_total: i64 = 0;

        let today = self.today_table();

        let _ = statement.query_row([&today], |row| Ok({
            let value: i64 = row.get(0).unwrap();
            last_total = value;
        }));

        return last_total > 0;
    }

    fn create_today_table(&self) {
        let query = format!("
        CREATE TABLE {} (timestamp INTEGER, from_ip TEXT, from_dns TEXT, to_ip TEXT, to_dns TEXT, packet_size INTEGER, payload_size INTEGER);
        ", self.today_table());

        self.connection.execute(&query, []).unwrap();

        let mut bmut = self.last_today.borrow_mut();
        *bmut = self.today_table();
    }
}

impl Logger for SQLiteLogger {
    fn log_traffic(
        &mut self,
        timestamp: i64,
        from_ip: &str,
        from_dns: &str,
        to_ip: &str,
        to_dns: &str,
        packet_size: i64,
        payload_size: i64,
    ) -> bool {
        // TODO: Queue up multiple logs into one write.
        println!(
            "[log_traffic] {} ({}) -> {} ({}). Sizes: {} ({})",
            from_ip, from_dns, to_ip, to_dns, packet_size, payload_size
        );

        if *self.last_today.borrow() != self.today_table() {
            self.setup_table();
        }

        let query = format!(
            "INSERT INTO {} VALUES (?, ?, ?, ?, ?, ?, ?);",
            self.today_table()
        );

        let mut statement = self.connection.prepare(&query).unwrap();

        let result = statement.execute(params![
          &timestamp,
          from_ip,
          from_dns,
          to_ip,
          to_dns,
          &packet_size,
          &payload_size  
        ]);

        if let Ok(_) = result {
            return true;
        }

        return false;
    }
}
