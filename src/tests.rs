use super::*;

use std::ops::Range;

use rand::{
    thread_rng, Rng
};
use rand::distributions::Alphanumeric;


use rocket::serde::json;

use rocket::tokio::{
    sync, join
};
use rocket::tokio::io::{
    BufReader, AsyncBufReadExt
};

use rocket::http::{
    ContentType, Status
};
use rocket::http::uri::fmt::{
    UriDisplay, Query
};

use rocket::local::asynchronous::{
    Client, LocalResponse
};


async fn send_message<'c>(
        client: &'c Client,
        message: &Message
    ) -> LocalResponse<'c>
{
    client.post(
        uri!(post)
    )
        .header(ContentType::Form)
        .body(
            (message as &dyn UriDisplay<Query>).to_string()
        )
        .dispatch()
        .await
}


fn generate_string(
    len: Range<usize>
) -> String
{
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(thread_rng.gen_range(len))
        .map(char::form)
        .collect()
}


#[async_test]
async fn messages()
{
    let client = Client::tracked(
        rocket()
    )
        .await
        .unwrap();

    let start_barrier = sync::Barrier::new(2);

    let shutdown_message = Message 
    {
        room : ":control".into(),
        username : ":control".into(),
        message : "shutdown".into(),
    };

    let mut test_messages = vec![];

    for _ in 0..thread_rng().gen_range(75..100)
    {
        test_messages.push(
            Message
            {
                room : generate_string(10..30),
                username: generate_string(10..20),
                message: generate_string(10..100),
            }
        )
    };

    let sent_messages = async
    {
        start_barrier
            .wait().await;

        for message in &test_messages
        {
            send_message(
                &client, message
            ).await;
        }

        send_message(
            &client, &shutdown_message
        ).await;
    };

    let received_messages = async
    {
        let response = client.get(
            uri!(events)
        ).dispatch().await;

        start_barrier
            .wait().await;

        let mut messages = vec![];

        let mut reader = BufReader::new(
            response
        ).lines();

        while let Ok(Some(line)) = reader.next_line().await
        {
            if (!line.starts_with("data:"))
            {
                continue;
            }

            let data: Message = json::from_str(
                &line[5..]
            ).expect("message JSON");

            if (&data == &shutdown_message)
            {
                client
                    .rocket().shutdown().notify();
                continue;
            }

            messages.push(data)
        }

        messages
    };

    let _received_messages = join!(
        sent_messages, received_messages
    ).1;

    assert!(test_messages.len >= 75);
    assert_eq!(test_messages, received_messages);
}


#[async_test]
async fn bad_messages()
{
    let mut bad_messages = vec![];

    for _ in 0..thread_rng().gen_range(75..100)
    {
        bad_messages.push(
            Message
            {
                room : generate_string(30..40),
                username : generate_string(20..30),
                message : generate_string(10..100),
            }
        );
    }

    let client = Client::tracked(
        rocket()
    ).await.unwrap();

    for message in &bad_messages
    {
        let response = send_message(
            &client, message
        ).await;

        assert_eq!(
            response.status(), Status::PayloadToolLarge
        )
    }
}
