//! Some tests to make sure networking is working on the host/container.
//! If these fail, check access permission/rights, especially when running in containers.

#[test]
pub fn udp_rx_tx_std() {
    use std::net::UdpSocket;

    let tx = UdpSocket::bind("0.0.0.0:8000").unwrap();
    let rx = UdpSocket::bind("0.0.0.0:8001").unwrap();

    tx.connect(rx.local_addr().unwrap()).unwrap();

    // when
    tx.send("Hello World".as_bytes()).unwrap();

    let mut rx_buffer = [0; 11];
    rx.recv(&mut rx_buffer).unwrap();

    assert_eq!(&rx_buffer[..], b"Hello World");
}

#[test]
pub fn udp_rx_tx_tokio() {
    use tokio::net::UdpSocket;

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let tx = UdpSocket::bind("0.0.0.0:8000").await.unwrap();
        let rx = UdpSocket::bind("0.0.0.0:8001").await.unwrap();

        tx.connect(rx.local_addr().unwrap()).await.unwrap();

        // when
        tx.send("Hello World".as_bytes()).await.unwrap();

        let mut rx_buffer = [0; 11];
        rx.recv(&mut rx_buffer).await.unwrap();

        assert_eq!(&rx_buffer[..], b"Hello World");
    });
}
