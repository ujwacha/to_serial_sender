// Mutex sucks Use MPSC channel	 


use std::env;
use std::io::Read;

use r2r::Publisher;

use serialport::SerialPort;

use r2r::QosProfile;

use tokio_stream::StreamExt;

use serde::Serialize;

use serde::Deserialize;


async fn publish_finding(
    mut rx: tokio::sync::mpsc::Receiver<u8>,
    publisher: Publisher<r2r::std_msgs::msg::Char>
)
where
{

    loop {

	let data = rx.recv().await.expect("Some error in recievind data");

	let msg = r2r::std_msgs::msg::Char {
	    data ,
	};
	
	let _ = publisher.publish(&msg);
    }
}




async fn serial_send(mut port: Box<dyn SerialPort>, mut connection: tokio::sync::mpsc::Receiver<u8> ) -> ! {
    // writes the data to port that we give 
    loop {
	// wait for data 
	let data = connection.recv().await.unwrap();


	let data = [data];
	
	// give data
	let _ = port.write(&data).unwrap();
    }
}


async fn serial_recieve(mut port: Box<dyn SerialPort>, connection: tokio::sync::mpsc::Sender<u8>) -> ! {
    // reads the data and sends this to something 
    let mut serial_buf: Vec<u8> = vec![0; 32];
    
    
    loop {
	port.read(serial_buf.as_mut_slice()).expect("NO_DATA");

	// serial_buf.iter().for_each(|c| async {
	//     connection.send(c).await.unwrap();
	// })

	for c in serial_buf.iter() {
	    connection.send(*c).await.unwrap();
	}
	
    }
}




// 
const NODENAME: &str = "Serial_Communication";
const NAMESPACE: &str = "";
const SUBESCRIBER: &str = "Serial_Sub";
const PUBLISHER : &str = "Serial_Pub";
const BUD_RATE: u32 = 9600;
const SERIAL_PORT_NAME: &str = "/dev/ttyACM0";
const SERIAL_PORT_TIMEOUT_MILLIS: u64 = 1500;


#[derive(Serialize,Deserialize)]
struct NodeInfo {
    nodename: String,
    namespace: String,
    subescriber: String,
    publisher: String,
    serial_port_name: String,
    bud_rate: u32,
    serial_port_timeout_millis: u64,
}

impl NodeInfo {
    fn from_args() -> Self {

	let args: Vec<String> = env::args().collect();

	if args.len() != 2 {
	    println!("something wrong with the arguments\n using the default");

	    Self {
		nodename: String::from(NODENAME),
		namespace: String::from(NAMESPACE),
		subescriber: String::from(SUBESCRIBER),
		publisher: String::from(PUBLISHER),
		serial_port_name: String::from(SERIAL_PORT_NAME),
		bud_rate: BUD_RATE,
		serial_port_timeout_millis: SERIAL_PORT_TIMEOUT_MILLIS,
	    }

	} else {
	    let filename = args.last().expect("something wrong with second argument");

	    let mut file = std::fs::File::open(filename).expect("could not open the file");

	    let mut contents = String::new();

	    let _ = file.read_to_string(&mut contents);

	    serde_json::from_str(&contents).expect("couldn't desearilize the file into struct. Is something wrong with your file?")

	}

	
    }


}


fn help_page() {
    println!("HELP PAGE: ");
    println!("to_serial_sender <filename of file containing json of configuration>");

    println!(
	"
    NodeInfo
	nodename: 
	namespace:
	subescriber:
	publisher: 
	serial_port_name:
	bud_rate:
	serial_port_timeout_millis:
        "
    );


}



#[tokio::main]
async fn main()  -> Result<(), Box<dyn std::error::Error>> {


    help_page();

    let nodeinfo = NodeInfo::from_args();
    
    // now work with serial
    let ctx = r2r::Context::create().unwrap();

    let mut node = r2r::Node::create(ctx, &nodeinfo.nodename, &nodeinfo.namespace).unwrap();

    let mut subscriber = node.subscribe::<r2r::std_msgs::msg::Char>(&nodeinfo.subescriber, QosProfile::default()).unwrap();

    let publisher = node.create_publisher::<r2r::std_msgs::msg::Char>(&nodeinfo.publisher, QosProfile::default()).unwrap();
    
//    let mut timer = node.create_wall_timer(std::time::Duration::from_millis(1000))?;




    // Serial Port


    let serial = serialport::new(&nodeinfo.serial_port_name, nodeinfo.bud_rate)
        .timeout(std::time::Duration::from_micros(nodeinfo.serial_port_timeout_millis))
        .open()
        .expect("Couldn't open the serial device");

    let serial2 = serial.try_clone().expect("Couldn't clone serial");

   // was editing here 


    let (tx_for_sending_data, rx_for_sending_data) = tokio::sync::mpsc::channel(1);
    let (tx_for_getting_data, rx_for_getting_data) = tokio::sync::mpsc::channel(1);


    
    tokio::spawn(serial_send(serial2, rx_for_sending_data));

    tokio::spawn(serial_recieve(serial, tx_for_getting_data));


    tokio::spawn(publish_finding(rx_for_getting_data, publisher));


   
    while let Some(value) = subscriber.next().await {

	let byte: u8 ;


	unsafe {
        // Use transmutation to convert Char to u8
            byte = std::mem::transmute::<r2r::std_msgs::msg::Char, u8>(value);
	}
	
	let _ = tx_for_sending_data.send(byte).await;
    }
    
    

   
    Ok(())
}
