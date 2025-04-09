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

pub fn remap (packet: & Ipv4Packet, connections: & mut Vec<Connection>, nat_ip_alice: Ipv4Addr, nat_ip_bob: Ipv4Addr) -> Option<Ipv4Packet<'static>> {
    // only need to remap the source IP. Check to see which IP to use
    let alice_subnet: Ipv4Network = "192.168.1.0/24".parse().unwrap();
    let nat_ip: Ipv4Addr = if alice_subnet.contains(packet.get_source()) {
        nat_ip_bob
    } else {
        nat_ip_alice
    };
    // if let Some(new_packet) = connections::remap(packet, & mut connections_vec, natIP_alice, natIP_bob)
    if let Some(old_source_port, new_tcp_packet) = remap_tcp(packet) {
        // TODO replace tcp in the packet
        // could just create a new Ipv4 header with the Ipv4Packet::populate() function and just modify the source?
        // TODO create the whole packet


        // debugging
        println!("Adding connection: {}:{} ({}:{}) -> {}:{}",
            packet.get_source(), tcp.get_source(),
            natIP, new_port,
            packet.get_destination(), tcp.get_destination()
        );
        
        connections.push(Connection {
                source_ip: packet.get_source(),
                source_port: new_tcp_packet.get_source(),
                destination_ip: packet.get_destination(),
                destination_port: new_tcp_packet.get_destination(),
                remapped_source_ip: natIP,
                remapped_destination_port: old_source_port,
        });
        // more debugging
        println!("Total connections: {}", connections.len());
    }
}

pub fn remap_tcp (packet: & Ipv4Packet) -> (u16, Option<TcpPacket<'static>>) {
//pub fn remap_tcp (packet: & Ipv4Packet, connections: & mut Vec<Connection>, natIP_alice: Ipv4Addr, natIP_bob: Ipv4Addr) -> Option<TcpPacket<'static>> {
    
    if let Some(tcp) = TcpPacket::new(packet.payload()) {
        // track old source port
        let old_source_port = tcp.get_source();
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
        // TODO new checksum

        // Create a new TcpPacket that owns the buffer to prevent lifetime issues and returns it
        Some(old_source_port, (TcpPacket::owned(buffer)
            .expect("Failed to create TcpPacket from buffer")))
    } else {
        None
    }


}

/*
pub fn unmap_tcp(packet: &Ipv4Packet, connections: Arc<Vec<Connection>>) -> MutableTcpPacket {
    // TODO
}
*/
