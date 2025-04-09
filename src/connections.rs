use std::net::{Ipv4Addr, SocketAddrV4};
use pnet::packet::Packet;
use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use pnet::packet::ipv4::Ipv4Packet;
use rand::Rng;
use std::sync::Arc;
use ipnetwork::Ipv4Network;

pub struct Connection {
    source_ip: Ipv4Addr,
    source_port: u16,
    destination_ip: Ipv4Addr,
    destination_port: u16,
    remapped_source_ip: Ipv4Addr,
    remapped_destination_port: u16,
}

pub fn remap_tcp (packet: &Ipv4Packet, connections: Arc<Vec<Connection>>, natIP_alice: Ipv4Addr, natIP_bob: Ipv4Addr) -> MutableTcpPacket{
    /* 
    let source_ip = packet.get_source();
    let destination_ip = packet.get_destination();
    let tcp_packet = TcpPacket::new(packet.payload())
        .expect("Failed to create TCP packet from payload");
    let source_port = tcp_packet.get_source();
    let destination_port = tcp_packet.get_destination();
    */
    // only need to remap the source IP. Check to see which IP to use
    let alice_subnet: Ipv4Network = "192.168.1.0/24".parse().unwrap();
    if alice_subnet.contains(packet.get_source()) {
        let natIP = natIP_bob;
    } else {
        let natIP = natIP_alice;
    }
    // generate new random port number
    let new_port = rand::thread_rng().gen_range(49152..65535);

    // generate MutableTcpPacket
    if let Some(tcp) = TcpPacket::new(packet.payload()) {
        let mut buffer = vec![0u8; tcp.packet().len()];
        let mut mutable_tcp = MutableTcpPacket::new(&mut buffer)
            .expect("Failed to create mutable TCP packet");
        mutable_tcp.clone_from(&tcp);
        // set the new port
        mutable_tcp.set_source(new_port);
        // set the destination port
        mutable_tcp.set_destination(new_port);
        
        // debugging
        println!("Adding connection: {}:{} ({}:{}) -> {}:{}",
            packet.get_source(), tcp.get_source(),
            natIP, new_port,
            packet.get_destination(), tcp.get_destination()
        );
        if let Some(connections_vec) = Arc::into_inner(connections) {
            connections_vec.append(Connection {
                source_ip: packet.get_source(),
                source_port: tcp.get_source(),
                destination_ip: packet.get_destination(),
                destination_port: tcp.get_destination(),
                remapped_source_ip: natIP,
                remapped_destination_port: new_port,
            });
            println!("Total connections: {}", connections_vec.len());
        } else {
            println!("Failed to add connection");
        };
        return mutable_tcp;
    }


}

pub fn unmap_tcp(packet: &Ipv4Packet, connections: Arc<Vec<Connection>>) -> MutableTcpPacket {
    // TODO
}
/* 
*if let Some(tcp) = TcpPacket::new(packet.payload()) {
        println!("TCP Packet {}:{} -> {}:{}",
            packet.get_source(), tcp.get_source(),
            packet.get_destination(), tcp.get_destination()
        );
*/
