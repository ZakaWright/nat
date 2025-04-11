use pnet::datalink;
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::Packet;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ip::IpNextHeaderProtocols;
//use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use std::thread;
use std::net::{Ipv4Addr, IpAddr};
use std::sync::{Arc, Mutex};
use pnet::transport::{transport_channel, TransportChannelType::Layer3};

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
        println!("Starting listener for Alice on {} ({})", interface_alice.name, interface_alice.ips[0]);
        start_listener(&interface_alice, connections_alice);
    });
    let handle_bob = thread::spawn(move || {
        println!("Starting listener for Bob on {} ({})", interface_bob.name, interface_bob.ips[0]);
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
                //println!("Packet received on {}", interface.name);
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
                // reads the protocol type
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
    let nat_ip_alice: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 5);
    let nat_ip_bob: Ipv4Addr = Ipv4Addr::new(10, 0, 1, 5);
    // determine if already mapped
    if let Ok(mut connections_vec) = connections.lock() {
        if packet.get_source() == nat_ip_alice || packet.get_destination() == nat_ip_alice || packet.get_source() == nat_ip_bob || packet.get_destination() == nat_ip_bob {
            if packet.get_destination() == nat_ip_alice || packet.get_destination() == nat_ip_bob {
                // already mapped
                if let Some(new_packet) = connections::unmap(packet, & mut connections_vec) {
                    send_packet_tcp(new_packet);
                } else {
                    send_packet_tcp(packet.clone());
                }
        } else {
            if let Some(new_packet) = connections::remap(packet, & mut connections_vec, nat_ip_alice, nat_ip_bob) {
                send_packet_tcp(new_packet);
            }
        }
    }
}

fn send_packet_tcp (packet: Ipv4Packet<'static>) -> std::io::Result<()> {
    // based on generated prompt from ChatGPT
    // the transport channel has to set a value for an rx buffer. 1500 is from Ethernet MTU
    // the layer 3 parameter sets the provided input to Layer3 containint a TCP packet
    let (mut tx, _) = transport_channel(1500, Layer3(IpNextHeaderProtocols::Tcp))?;
    tx.send_to(&packet, IpAddr::V4(packet.get_destination()))?;
    Ok(())
}