use ergot::toolkits::tokio_udp::EdgeStack;
use ergot::well_known::DeviceInfo;
use tokio::select;

pub async fn basic_services(stack: EdgeStack, port: u16) {
    let info = DeviceInfo {
        name: Some("OperatorUI".try_into().unwrap()),
        description: Some(
            "MakerPnP - Operator UI"
                .try_into()
                .unwrap(),
        ),
        unique_id: port.into(),
    };
    let do_pings = stack.services().ping_handler::<4>();
    let do_info = stack
        .services()
        .device_info_handler::<4>(&info);

    select! {
        _ = do_pings => {}
        _ = do_info => {}
    }
}
