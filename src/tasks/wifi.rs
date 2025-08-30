use core::net::Ipv4Addr;
use core::str::FromStr;
use defmt::{info, error};
use embassy_net::StaticConfigV4;
use embassy_net::Ipv4Cidr;
use anyhow::anyhow;

use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use esp_hal_dhcp_server::simple_leaser::SingleDhcpLeaser;
use esp_hal_dhcp_server::structs::DhcpServerConfig;
use esp_wifi::{
    wifi::{self, WifiController, WifiDevice, WifiEvent, WifiState},
    EspWifiController,
};

use crate::mk_static;


// Unlike Station mode, You can give any IP range(private) that you like
// IP Address/Subnet mask eg: STATIC_IP=192.168.13.37/24
const STATIC_IP: &str = "192.168.2.2/24";
// Gateway IP eg: GATEWAY_IP="192.168.13.37"
const GATEWAY_IP: &str = "192.168.2.1";

const SSID: &str = "Airsoft";
const PASSWORD: &str = "Airsoft123";

pub async fn start_wifi(
    esp_wifi_ctrl: &'static EspWifiController<'static>,
    wifi: esp_hal::peripherals::WIFI<'static>,
    mut rng: Rng,
    spawner: &Spawner,
) -> anyhow::Result<Stack<'static>> {
    let (controller, interfaces) = esp_wifi::wifi::new(esp_wifi_ctrl, wifi).unwrap();
    let wifi_interface = interfaces.ap;
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    // Parse STATIC_IP
    let ip_addr =
        Ipv4Cidr::from_str(STATIC_IP).map_err(|_| anyhow!("Invalid STATIC_IP: {}", STATIC_IP))?;

    // Parse GATEWAY_IP
    let gateway = Ipv4Addr::from_str(GATEWAY_IP)
        .map_err(|_| anyhow!("Invalid GATEWAY_IP: {}", GATEWAY_IP))?;

    // Create Network config with IP details
    let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: ip_addr,
        gateway: Some(gateway),
        dns_servers: Default::default(),
    });

    // alternate approach
    // let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
    //     address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 2, 1), 24),
    //     gateway: Some(Ipv4Address::from_bytes(&[192, 168, 2, 1])),
    //     dns_servers: Default::default(),
    // });

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        net_seed,
    );

    spawner.spawn(connection_task(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    wait_for_connection(stack).await;

    Ok(stack)
}

async fn wait_for_connection(stack: Stack<'_>) {
    info!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Connect to the AP `esp-wifi` and point your browser to http://{}/", STATIC_IP);
    while !stack.is_config_up() {
        Timer::after(Duration::from_millis(100)).await
    }
    stack
        .config_v4()
        .inspect(|c| info!("ipv4 config: {:?}", defmt::Debug2Format(&c)));
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    info!("start connection task");
    loop {
        info!("loop start");
        if esp_wifi::wifi::wifi_state() == WifiState::ApStarted {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(), // Set your password
                auth_method: esp_wifi::wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started!");
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
pub async fn dhcp_server(stack: Stack<'static>) {
    let config = DhcpServerConfig {
        ip: Ipv4Addr::new(192, 168, 2, 1),
        lease_time: Duration::from_secs(3600),
        gateways: &[Ipv4Addr::new(192, 168, 2, 1)],
        subnet: None,
        dns: &[Ipv4Addr::new(192, 168, 2, 1)],
        use_captive_portal: true,
    };

    /*
    let mut leaser = SimpleDhcpLeaser {
        start: Ipv4Addr::new(192, 168, 2, 50),
        end: Ipv4Addr::new(192, 168, 2, 200),
        leases: Default::default(),
    };
    */
    let mut leaser = SingleDhcpLeaser::new(Ipv4Addr::new(192, 168, 2, 69));

    let res = esp_hal_dhcp_server::run_dhcp_server(stack, config, &mut leaser).await;
    if let Err(e) = res {
        error!("DHCP SERVER ERROR: {:?}", defmt::Debug2Format(&e));
    }
}
