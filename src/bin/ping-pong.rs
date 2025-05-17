use capataz::Context;
use capataz::EventListener;
use capataz::prelude::*;
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use std::sync::Arc;


struct Ping {
    sender: mpsc::Sender<Pong>,
}

struct Pong {
    sender: mpsc::Sender<Ping>,
}

fn init_worker(ping_sender0: mpsc::Sender<Ping>, pong_sender0: mpsc::Sender<Pong>) -> Node {
    worker::Spec::new("init", vec![worker::with_restart(worker::Restart::Temporary)], move |_ctx| {
	let ping_sender1 = ping_sender0.clone();
	let pong_sender1 = pong_sender0.clone();
	async move {
	    let ping_sender = ping_sender1.clone();
	    let pong_sender = pong_sender1.clone();
	    let _ = ping_sender.send(Ping{sender: pong_sender}).await;
	    Ok(())
	}
    })
}
   
fn ping_worker(sender_chan0: mpsc::Sender<Ping>, receiver_chan0: mpsc::Receiver<Ping>) -> Node {
    let sender_chan1 = sender_chan0.clone();
    let receiver_chan1 = Arc::new(Mutex::new(receiver_chan0));
    worker::Spec::new("ping", vec![], move |ctx| {
	let sender_chan2 = sender_chan1.clone();
	let receiver_chan2 = receiver_chan1.clone();
	async move {
	    let receiver_chan3 = receiver_chan2.clone();
	    let mut receiver_chan = receiver_chan3.lock().await;
	    loop {
		tokio::select! {
		    _ = ctx.done() => {
			println!("ping done!");
			break
		    }
		    msg0 = (*receiver_chan).recv() => match msg0 {
			Some(msg) => {
			    println!("received ping!");
			    let _ = time::sleep(time::Duration::from_secs(3)).await;
			    println!("sending pong!");
			    tokio::select! {
				_ = ctx.done() => {
				    println!("could not send pong");
				}
				_ = msg.sender.send(Pong{sender: sender_chan2.clone()}) => {}
			    }
			},
			None => {
			    println!("didn't receive ping")
			},
		    }
		}
	    } 
	    Ok(())
	}
    })
}

fn pong_worker(sender_chan0: mpsc::Sender<Pong>, receiver_chan0: mpsc::Receiver<Pong>) -> Node {
    let sender_chan1 = sender_chan0.clone();
    let receiver_chan1 = Arc::new(Mutex::new(receiver_chan0));
    worker::Spec::new("pong", vec![], move |ctx| {
	let sender_chan2 = sender_chan1.clone();
	let receiver_chan2 = receiver_chan1.clone();
	async move {
	    let receiver_chan3 = receiver_chan2.clone();
	    let mut receiver_chan = receiver_chan3.lock().await;
	    loop {
		tokio::select! {
		    _ = ctx.done() => {
			println!("pong done!");
			break
		    }
		    msg0 = (*receiver_chan).recv() => match msg0 {
			Some(msg) => {
			    println!("received pong!");
			    let _ = time::sleep(time::Duration::from_secs(3)).await;
			    println!("sending ping!");
			    tokio::select! {
				_ = ctx.done() => {
				    println!("could not send ping");
				}
				_ = msg.sender.send(Ping{sender: sender_chan2.clone()}) => {}
			    }
			},
			None => {
			    println!("didn't receive pong")
			},
		    }
		}
	    } 
	    Ok(())
	}
    })
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let (ctx, _abort) = Context::new().with_cancel();
    let event_listener = EventListener::new(|ev| async move {
	println!("EVENT: {:?}", ev)
    });
    let sup_spec = supervisor::Spec::new("root", vec![], move |_ctx| {
	let (ping_sender, ping_receiver) = mpsc::channel(1);
	let (pong_sender, pong_receiver) = mpsc::channel(1);
	let ping = ping_worker(ping_sender.clone(), ping_receiver);
	let pong = pong_worker(pong_sender.clone(), pong_receiver);
	let init = init_worker(ping_sender, pong_sender);
	vec![init, ping, pong]
    });
    let result = sup_spec.start(ctx, event_listener).await;
    match result {
	Ok(sup) => {
	    let _ = time::sleep(time::Duration::from_secs(10)).await;
	    println!("finishing application");
	    let (result, _spec) = sup.terminate().await;
	    match result {
		Ok(_spec) => println!("application finished without errors"),
		Err(err) => println!("application finished with error: {}", err),
	    }
	},
	Err(err) => {
	    println!("application could not start due to error: {}", err);
	    return
	},
    }
}
