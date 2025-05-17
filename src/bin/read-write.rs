use capataz::Context;
use capataz::EventListener;
use capataz::prelude::*;
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use std::sync::Arc;


fn read_worker(receiver_chan0: mpsc::Receiver<String>) -> Node {
    let receiver_chan1 = Arc::new(Mutex::new(receiver_chan0));
    worker::Spec::new("reader", vec![], move |ctx| {
	let receiver_chan2 = receiver_chan1.clone();
	async move {
	    let receiver_chan3 = receiver_chan2.clone();
	    let mut receiver_chan = receiver_chan3.lock().await;
	    loop {
		tokio::select! {
		    _ = ctx.done() => {
			println!("reader done!");
			break
		    }
		    msg = (*receiver_chan).recv() => {
			println!("receiving message {:?}", msg)
		    }
		}
	    } 
	    Ok(())
	}
    })
}

fn write_worker(sender_chan0: mpsc::Sender<String>) -> Node {
    worker::Spec::new("writer", vec![], move |ctx| {
	let mut interval = time::interval(time::Duration::from_secs(3));
	let sender_chan = sender_chan0.clone();
	async move {
	    loop {
		tokio::select! {
		    _ = ctx.done() => {
			println!("writer done!");
			break
		    }
		    _ = interval.tick() => {
			tokio::select! {
			    _ = ctx.done() => {
				println!("could not send message");
				break
			    }
			    _ = sender_chan.send("hello from writer".to_string()) => {
				continue
			    }
			}
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
	let (sender, receiver) = mpsc::channel(10);
	let writer = write_worker(sender);
	let reader = read_worker(receiver);
	vec![writer, reader]
    });
    let result = sup_spec.start(ctx, event_listener).await;
    match result {
	Ok(sup) => {
	    let _ = time::sleep(tokio::time::Duration::from_secs(10)).await;
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
