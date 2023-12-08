#[cfg(test)] mod tests;

#[macro_use] extern crate rocket;

use rocket::{
    State, Shutdown
};

use rocket::form::Form;

use rocket::fs::{
    relative, FileServer
};

use rocket::serde::{
    Serialize, Deserialize
};

use rocket::response::stream::{
    EventStream, Event
};

use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{
    channel, Sender, error::RecvError
};


#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
struct Message
{
    #[field(validate = len (..30))]
    pub room : String,
    #[field(validate = len (..20))]
    pub username : String,
    pub message : String,
}

#[ignore = ""]
#[get("/events")]
async fn events(
    queue: &State<Sender<Message>>,
    mut end: Shutdown 
) -> EventStream![]
{
    let mut rx = queue.subscribe();

    EventStream!
    {
        loop
        {
            let message = select!
            {
                message = rx.recv() => match message
                {
                    Ok(message) => message,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,

            };

            yield Event::json(&message);
        }
    }
}


#[post("/message", data = "<form>")]
fn post(
    form: Form<Message>,
    queue: &State<Sender<Message>>
)
{
    let _res = queue.send(
        form.into_inner()
    );
}


#[launch]
fn rocket() -> _ 
{
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount("/", routes![post, events])
        .mount("/", FileServer::from(relative!("static")))
}
