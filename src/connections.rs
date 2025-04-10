use std::net::{Ipv4Addr};
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
//use pnet::packet::ipv4;
use rand::Rng;
//use std::sync::Arc;
use pnet::util::checksum;
use ipnetwork::Ipv4Network;

pub struct Connection {
    source_ip: Ipv4Addr,
    source_port: u16,
    destination_ip: Ipv4Addr,
    destination_port: u16,
    remapped_source_ip: Ipv4Addr,
    remapped_source_port: u16,
}

pub fn remap (packet: & Ipv4Packet, connections: & mut Vec<Connection>, nat_ip_alice: Ipv4Addr, nat_ip_bob: Ipv4Addr) -> Option<Ipv4Packet<'static>> {
    // only need to remap the source IP. Check to see which IP to use
    let original_source_ip = packet.get_source();
    let alice_subnet: Ipv4Network = "192.168.1.0/24".parse().unwrap();
    let nat_ip: Ipv4Addr = if alice_subnet.contains(original_source_ip) {
        nat_ip_bob
    } else {
        nat_ip_alice
    };
    // if let Some(new_packet) = connections::remap(packet, & mut connections_vec, natIP_alice, natIP_bob)
    if let (Some(old_source_port), Some(new_tcp_packet)) = remap_tcp(packet, &nat_ip) {
        // replace tcp in the packet
        // get the length for the buffer
        // the * 4 is because the header length returns the number of words in the header, *4 will give the correct length
        let packet_length = packet.get_header_length() as usize * 4 + new_tcp_packet.packet().len() as usize;
        let mut buffer = vec![0u8; packet_length];
        // create the mutable packet
        let mut new_ip_packet = MutableIpv4Packet::new(&mut buffer)
            .expect("Failed to create mutable packet");
        // copy the headers
        new_ip_packet.clone_from(packet);
        // set the new Source IP
        new_ip_packet.set_source(nat_ip);
        new_ip_packet.set_total_length(packet_length as u16);
        // replace the next layer protocol
        new_ip_packet.payload_mut().copy_from_slice(new_tcp_packet.packet());
        
        // calculate and set the new IP checksum
        new_ip_packet.set_checksum(checksum(new_ip_packet.packet(), 0));

        // debugging
        println!("Adding connection: {}:{} ({}:{}) -> {}:{}",
            packet.get_source(), new_tcp_packet.get_source(),
            original_source_ip, old_source_port,
            packet.get_destination(), new_tcp_packet.get_destination()
        );
        
        connections.push(Connection {
                source_ip: packet.get_source(),
                source_port: old_source_port,
                destination_ip: packet.get_destination(),
                destination_port: new_tcp_packet.get_destination(),
                remapped_source_ip: nat_ip,
                remapped_source_port: new_tcp_packet.get_source(),
        });
        // more debugging
        println!("Total connections: {}", connections.len());

        Some(Ipv4Packet::owned(buffer)
            .expect("Failed to create Ipv4Packet"))
    } else {
        None
    }
}

pub fn remap_tcp (packet: & Ipv4Packet, nat_ip: &Ipv4Addr) -> (Option<u16>, Option<TcpPacket<'static>>) {
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
        // new checksum
        // adapted from ChatGPT prompt (not a lot of documentation or code samples for this)
        let mut tcp_psuedo_header = Vec::new();
        tcp_psuedo_header.extend_from_slice(&nat_ip.octets());
        tcp_psuedo_header.extend_from_slice(&packet.get_destination().octets());
        tcp_psuedo_header.push(0);
        tcp_psuedo_header.push(pnet::packet::ip::IpNextHeaderProtocols::Tcp.0);
        let tcp_length = (mutable_tcp.get_data_offset() as u16) * 4;
        tcp_psuedo_header.extend_from_slice(&tcp_length.to_be_bytes());
        tcp_psuedo_header.extend_from_slice(&mutable_tcp.packet());
        let new_checksum = checksum(&tcp_psuedo_header, 0);
        // set the new checksum
        mutable_tcp.set_checksum(new_checksum);

        // Create a new TcpPacket that owns the buffer to prevent lifetime issues and returns it
        (Some(old_source_port), Some(TcpPacket::owned(buffer)
            .expect("Failed to create TcpPacket from buffer")))
    } else {
        (None, None)
    }


}

pub fn unmap (packet: & Ipv4Packet, connections: & mut Vec<Connection>) -> Option<Ipv4Packet<'static>> {
    // find the connection
    if let Some(tcp) = TcpPacket::new(packet.payload()) {
        if let Some(connection) = connections.iter().find(|c|
            c.remapped_source_ip == packet.get_destination() &&
            c.remapped_source_port == tcp.get_destination()
        ) {
            // unmap the TCP packet
            if let Some(new_tcp_packet) = unmap_tcp(packet, connection) {
                // unmap the IP packet
                // replace tcp in the packet
                // get the length for the buffer
                let packet_length = packet.get_header_length() as usize * 4 + new_tcp_packet.packet().len() as usize;
                let mut buffer = vec![0u8; packet_length];
                // create the mutable packet
                let mut new_ip_packet = MutableIpv4Packet::new(&mut buffer)
                    .expect("Failed to create mutable packet");
                
                // copy the headers
                new_ip_packet.clone_from(packet);
                // remap the destination IP
                new_ip_packet.set_destination(connection.source_ip);
                new_ip_packet.set_total_length(packet_length as u16);
                // replace the next layer protocol
                new_ip_packet.payload_mut().copy_from_slice(new_tcp_packet.packet());
                
                // calculate and set the new IP checksum
                new_ip_packet.set_checksum(checksum(new_ip_packet.packet(), 0));

                // debugging
                println!("Remapping from connection: {}:{} ({}:{}) -> {}:{}",
                    packet.get_source(), new_tcp_packet.get_source(),
                    connection.source_ip, connection.source_port,
                    packet.get_destination(), new_tcp_packet.get_destination()
                );

                Some(Ipv4Packet::owned(buffer)
                    .expect("Failed to create Ipv4Packet"))
            } else {
                println!("Failed to unmap IP packet");
                None
            }
        } else {
            println!("Failed to find connection");
            None
        }
    } else {
        None
    }
}
pub fn unmap_tcp(packet: & Ipv4Packet, connection: &Connection) -> Option<TcpPacket<'static>> {
    if let Some(tcp) = TcpPacket::new(packet.payload()) {
        // create MutableTcpPacket
        let mut buffer = vec![0u8; tcp.packet().len()];
        let mut mutable_tcp = MutableTcpPacket::new(&mut buffer)
            .expect("Failed to create mutable TCP packet");
        mutable_tcp.clone_from(&tcp);
        // set the new port
        // mutable_tcp modifies the buffer directly
        mutable_tcp.set_destination(connection.source_port);
        // new checksum
        // adapted from ChatGPT prompt (not a lot of documentation or code samples for this)
        let mut tcp_psuedo_header = Vec::new();
        tcp_psuedo_header.extend_from_slice(&packet.get_source().octets());
        tcp_psuedo_header.extend_from_slice(&connection.source_ip.octets());
        tcp_psuedo_header.push(0);
        tcp_psuedo_header.push(pnet::packet::ip::IpNextHeaderProtocols::Tcp.0);
        let tcp_length = (mutable_tcp.get_data_offset() as u16) * 4;
        tcp_psuedo_header.extend_from_slice(&tcp_length.to_be_bytes());
        tcp_psuedo_header.extend_from_slice(&mutable_tcp.packet());
        let new_checksum = checksum(&tcp_psuedo_header, 0);
        // set the new checksum
        mutable_tcp.set_checksum(new_checksum);

        // Create a new TcpPacket that owns the buffer to prevent lifetime issues and returns it
        Some(TcpPacket::owned(buffer)
            .expect("Failed to create TcpPacket from buffer"))
    } else {
        None
    }
}