mod config {
    use serde::Deserialize;
    #[derive(Debug, Default, Deserialize)]
    pub struct ExampleConfig {
        pub server_addr: String,
        pub pg: deadpool_postgres::Config,
    }
}

mod models {
    use serde::{Deserialize, Serialize};
    use tokio_pg_mapper_derive::PostgresMapper;

    #[derive(Deserialize, PostgresMapper, Serialize)]
    #[pg_mapper(table = "coordinates")] // singular 'user' is a keyword..
    pub struct Coordinate {
        pub value: i16,
        pub axis: String,
    }
}

mod errors {
    use actix_web::{HttpResponse, ResponseError};
    use deadpool_postgres::PoolError;
    use derive_more::{Display, From};
    use tokio_pg_mapper::Error as PGMError;
    use tokio_postgres::error::Error as PGError;

    #[derive(Display, From, Debug)]
    pub enum MyError {
        NotFound,
        PGError(PGError),
        PGMError(PGMError),
        PoolError(PoolError),
    }
    impl std::error::Error for MyError {}

    impl ResponseError for MyError {
        fn error_response(&self) -> HttpResponse {
            match *self {
                MyError::NotFound => HttpResponse::NotFound().finish(),
                MyError::PoolError(ref err) => {
                    HttpResponse::InternalServerError().body(err.to_string())
                }
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    }
}

mod db {
    use deadpool_postgres::Client;
    use tokio_pg_mapper::FromTokioPostgresRow;

    use crate::{errors::MyError, models::Coordinate};

    pub async fn add_coordinate(client: &Client, coordinate_info: Coordinate) -> Result<Coordinate, MyError> {
        let _stmt = include_str!("../sql/add_coordinate.sql");
        let _stmt = _stmt.replace("$table_fields", &Coordinate::sql_table_fields());
        let stmt = client.prepare(&_stmt).await.unwrap();

        client
            .query(
                &stmt,
                &[
                    &coordinate_info.value,
                    &coordinate_info.axis,
                ],
            )
            .await?
            .iter()
            .map(|row| Coordinate::from_row_ref(row).unwrap())
            .collect::<Vec<Coordinate>>()
            .pop()
            .ok_or(MyError::NotFound) // more applicable for SELECTs
    }
}

mod handlers {
    use std::time::Duration;
    use actix_files::NamedFile;
    use actix_web::{get, web, Error, HttpResponse};
    use actix_web::rt::time;
    use deadpool_postgres::{Client, Pool};

    use crate::{db, errors::MyError, generate_coordinates, models::Coordinate};
    
    #[get("/")]
    pub async fn add_coordinate(
        db_pool: web::Data<Pool>,
        ) -> Result<NamedFile, Error> {
        let mut interval = time::interval(Duration::from_millis(10000));
    
            interval.tick().await;
            let coordinate = generate_coordinates(2.0, 0.1, 0.0, 10);
            for coord in coordinate.iter(){
                let coordinate_info_x = Coordinate {
                    value : coord.0 as i16,
                    axis : "x".parse()?
                };
                let coordinate_info_y = Coordinate {
                    value: coord.1 as i16,
                    axis: "y".parse()?
                };
                let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;
                let new_coordinate_x = db::add_coordinate(&client, coordinate_info_x).await?;
                let new_coordinate_y = db::add_coordinate(&client, coordinate_info_y).await?;
            }
        
        Ok(NamedFile::open("C:\\Users\\watakai\\Desktop\\actix\\postgres\\templates\\index.html")?)
        
    }
}

use ::config::Config;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use handlers::add_coordinate;
use tokio_postgres::NoTls;
use actix_files as fs;

use crate::config::ExampleConfig;

fn generate_coordinates(amplitude: f32, frequency: f32, phase: f32, num_points: i32) -> Vec<(f32, f32)> {
    let mut coordinates: Vec<(f32, f32)> = Vec::new();
    for i in 0..num_points {
        let x = i as f32;
        let y = amplitude * f32::sin(frequency * x + phase);
        coordinates.push((x, y));
    }
    coordinates
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let config_ = Config::builder()
        .add_source(::config::Environment::default())
        .build()
        .unwrap();

    let config: ExampleConfig = config_.try_deserialize().unwrap();

    let pool = config.pg.create_pool(None, NoTls).unwrap();

    let server = HttpServer::new(move || {
        App::new().service(fs::Files::new("/static", ".")
                .show_files_listing()
                .use_last_modified(true),)
            .app_data(web::Data::new(pool.clone()))
            .service(add_coordinate)
    })
    .bind(config.server_addr.clone())?
    .run();
    println!("Server running at http://{}/", config.server_addr);

    server.await
}
