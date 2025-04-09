use std::net::{Ipv4Addr};
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use pnet::packet::ipv4::Ipv4Packet;
use rand::Rng;
//use std::sync::Arc;
use ipnetwork::Ipv4Network;

pub struct Connection {
    source_ip: Ipv4Addr,
    source_port: u16,
    destination_ip: Ipv4Addr,
    destination_port: u16,
    remapped_source_ip: Ipv4Addr,
    remapped_destination_port: u16,
}

pub fn remap_tcp (packet: & Ipv4Packet, connections: & mut Vec<Connection>, natIP_alice: Ipv4Addr, natIP_bob: Ipv4Addr) -> Option<TcpPacket<'static>> {
    // only need to remap the source IP. Check to see which IP to use
    if let Some(tcp) = TcpPacket::new(packet.payload()) {
        let alice_subnet: Ipv4Network = "192.168.1.0/24".parse().unwrap();
        let natIP: Ipv4Addr = if alice_subnet.contains(packet.get_source()) {
            natIP_bob
        } else {
            natIP_alice
        };
        // generate new random port number
        let new_port = rand::thread_rng().gen_range(49152..65535);

        // create MutableTcpPacket
        let mut buffer = vec![0u8; tcp.packet().len()];
        let mut mutable_tcp = MutableTcpPacket::new(&mut buffer)
            .expect("Failed to create mutable TCP packet");
        mutable_tcp.clone_from(&tcp);
        // set the new port
        // mutable_tcp modifies the buffer directly
        mutable_tcp.set_source(new_port);

        // debugging
        println!("Adding connection: {}:{} ({}:{}) -> {}:{}",
            packet.get_source(), tcp.get_source(),
            natIP, new_port,
            packet.get_destination(), tcp.get_destination()
        );
        
        connections.push(Connection {
                source_ip: packet.get_source(),
                source_port: tcp.get_source(),
                destination_ip: packet.get_destination(),
                destination_port: tcp.get_destination(),
                remapped_source_ip: natIP,
                remapped_destination_port: new_port,
        });
        // more debugging
        println!("Total connections: {}", connections.len());

        // Create a new TcpPacket that owns the buffer to prevent lifetime issues and returns it
        Some(TcpPacket::owned(buffer)
            .expect("Failed to create TcpPacket from buffer"))
    } else {
        None
    }


}

/*
pub fn unmap_tcp(packet: &Ipv4Packet, connections: Arc<Vec<Connection>>) -> MutableTcpPacket {
    // TODO
}
*/
