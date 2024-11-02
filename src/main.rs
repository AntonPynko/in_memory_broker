use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{self, Sender, Receiver};
//use serde_json::Value;
use warp::sse::Event;
use warp::Filter;

type ClientId = u64;
type TopicName = String;
type Message = Vec<u8>;

#[derive(Debug, Clone)]
struct Topic {
    sender: Sender<Message>,
}

#[derive(Debug)]
struct Broker {
    topics: Arc<Mutex<HashMap<TopicName, Topic>>>,
    clients: Arc<Mutex<HashMap<ClientId, HashSet<TopicName>>>>,
}

impl Broker {
    fn new() -> Self {
        Broker {
            topics: Arc::new(Mutex::new(HashMap::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_topic(&self, name: TopicName) {
        let (sender, _receiver) = broadcast::channel(100);
        let topic = Topic { sender };
        self.topics.lock().unwrap().insert(name, topic);
    }

    fn subscribe(&self, client_id: ClientId, topic_name: &TopicName) -> Option<Receiver<Message>> {
        let topics = self.topics.lock().unwrap();
        if let Some(topic) = topics.get(topic_name) {
            let receiver = topic.sender.subscribe();
            self.clients.lock().unwrap().entry(client_id).or_default().insert(topic_name.clone());
            Some(receiver)
        } else {
            None
        }
    }

    fn is_subscribed(&self, client_id: ClientId, topic_name: &TopicName) -> bool {
        self.clients.lock().unwrap()
            .get(&client_id)
            .map_or(false, |topics| topics.contains(topic_name))
    }

    fn unsubscribe(&self, client_id: ClientId, topic_name: &TopicName) {
        let mut clients = self.clients.lock().unwrap();
        if let Some(topics) = clients.get_mut(&client_id) {
            topics.remove(topic_name);
            if topics.is_empty() {
                clients.remove(&client_id);
            }
        }
    }

    fn publish(&self, topic_name: &TopicName, message: Message) {
        if let Some(topic) = self.topics.lock().unwrap().get(topic_name) {
            let _ = topic.sender.send(message);
        }
    }
}

async fn sse_handler(broker: Arc<Broker>, client_id: ClientId, topic_name: TopicName, mut receiver: Receiver<Message>) -> impl warp::Reply {
    let stream = async_stream::stream! {
        while let Ok(message) = receiver.recv().await {
            if broker.is_subscribed(client_id, &topic_name) {
                yield Ok::<_, warp::Error>(Event::default().data(String::from_utf8_lossy(&message).into_owned()));
            } else {
                break;
            }
        }
    };
    warp::sse::reply(warp::sse::keep_alive().stream(stream))
}

#[tokio::main]
async fn main() {
    let broker = Arc::new(Broker::new());

    // Создание маршрутов Warp для SSE и REST-операций
    let broker_filter = warp::any().map(move || broker.clone());

    let create_topic_route = warp::path!("topic" / String)
        .and(warp::post())
        .and(broker_filter.clone())
        .map(|name: String, broker: Arc<Broker>| {
            broker.create_topic(name);
            warp::reply::with_status("Topic created", warp::http::StatusCode::CREATED)
        });

    let publish_message_route = warp::path!("topic" / String / "message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(broker_filter.clone())
        .map(|topic_name: String, body: bytes::Bytes, broker: Arc<Broker>| {
            broker.publish(&topic_name, body.to_vec());
            warp::reply::with_status("Message published", warp::http::StatusCode::OK)
        });

    let subscribe_route = warp::path!("topic" / String / "subscribe" / u64)
        .and(warp::get())
        .and(broker_filter.clone())
        .and_then(|topic_name: String, client_id: ClientId, broker: Arc<Broker>| async move {
            if let Some(receiver) = broker.subscribe(client_id, &topic_name) {
                // Передаем `broker`, `client_id` и `topic_name` в `sse_handler`
                Ok::<_, warp::Rejection>(sse_handler(broker, client_id, topic_name, receiver).await)
            } else {
                Err(warp::reject::not_found())
            }
        });

    let unsubscribe_route = warp::path!("topic" / String / "unsubscribe" / u64)
    .and(warp::post())
    .and(broker_filter.clone())
    .map(|topic_name: String, client_id: ClientId, broker: Arc<Broker>| {
        broker.unsubscribe(client_id, &topic_name);
        warp::reply::with_status("Unsubscribed", warp::http::StatusCode::OK)
    });

    let routes = create_topic_route
        .or(publish_message_route)
        .or(subscribe_route)
        .or(unsubscribe_route);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
