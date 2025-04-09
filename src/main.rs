use pnet::datalink;
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::Packet;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ip::IpNextHeaderProtocols;
//use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use std::thread;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};

mod connections;


fn main() {
    // read available interfaces
    let interfaces = datalink::interfaces();
    // set alice and bob interfaces
    // TODO set up somethng to read the interfaces and automatically set them properly
    let interface_alice = interfaces[2].clone();
    let interface_bob = interfaces[3].clone();
    
    // Connections vector
    let connections_arc: Arc<Mutex<Vec<connections::Connection>>> = Arc::new(Mutex::new(Vec::new()));
    let connections_alice = Arc::clone(&connections_arc);
    let connections_bob = Arc::clone(&connections_arc);

    // Start threads to handle multiple listeners
    let handle_alice = thread::spawn(move || {
        start_listener(&interface_alice, connections_alice);
    });
    let handle_bob = thread::spawn(move || {
        start_listener(&interface_bob, connections_bob);
    });
    
    // Join the handles to main to keep threads open
    handle_alice.join().unwrap();
    handle_bob.join().unwrap();

}


fn start_listener(interface: &datalink::NetworkInterface, connections: Arc<Mutex<Vec<connections::Connection>>>) {
    // adapted from "Tutorial: Capturing Network Packets with pnet in Rust" by Cyprien Avico

    // reads the tx and rx objects from the datalink channel for the interface
    let (_, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type: {}", & interface),
        Err(e) => panic!("An error occured when creating the datalink channel: {}", e),
    };

    println!("Start reading packets on {} ({})", interface.name, interface.ips[0]);
    // will read packets until exit
    loop {
        match rx.next() {
            Ok(packet) => {
                // create new handle to the connections vector
                let connections = Arc::clone(&connections);
                // reads layer 2 from the packet
                // unrwap prevents empty values from creating an error, I think
                let ethernet_packet = EthernetPacket::new(packet).unwrap();
                process_packets(&ethernet_packet, connections);
            },
            Err(e) => {
                panic!("An error occured while reading: {}", e);
            }
        }
    }
}

fn process_packets(ethernet_packet: &EthernetPacket, connections: Arc<Mutex<Vec<connections::Connection>>>) {
    // read the Ethernet Type
    match ethernet_packet.get_ethertype() {
        EtherTypes::Ipv4 => {
            // convert layer 2 packet to Ipv4 packet
            if let Some(ipv4_packet) = Ipv4Packet::new(ethernet_packet.payload()) {
                // reads the protocol
                match ipv4_packet.get_next_level_protocol() {
                    IpNextHeaderProtocols::Icmp => {
                        println!("ICMP: {} -> {}", 
                            ipv4_packet.get_source(), ipv4_packet.get_destination());
                    }
                    IpNextHeaderProtocols::Tcp => {                        
                        process_tcp(&ipv4_packet, connections);
                    }
                    _ => {
                        // handle all other protocols
                    }
                }
            }
        }
        _  => {
            // Handle all other EtherTypes (eg Ipv6)
        }
    }
}

fn process_tcp(packet: &Ipv4Packet, connections: Arc<Mutex<Vec<connections::Connection>>>) {
    let natIP_alice: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 5);
    let natIP_bob: Ipv4Addr = Ipv4Addr::new(10, 0, 1, 5);
    // determine if already mapped
    if packet.get_source() == natIP_alice || packet.get_destination() == natIP_alice || packet.get_source() == natIP_bob || packet.get_destination() == natIP_bob {
        println!("Already mapped");
        //let new_packet = connections::unmap_tcp(packet, connections);
    } else {
        if let Ok(mut connections_vec) = connections.lock() {
            if let Some(new_packet) = connections::remap_tcp(packet, & mut connections_vec, natIP_alice, natIP_bob) {
                // send packet
            }
        }
    }
}