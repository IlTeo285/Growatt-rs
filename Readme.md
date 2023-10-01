# Growatt-rs

This project is a minimal and work-in-progress interface with Growatt APIs to retrive PV infos.

## Usage

``` rust
let mut client = GrowattServer::new();
client.login(<username>, <password>).await.unwrap();

let res = client.device_list_by_plant(<plant-id>).await.unwrap();
println!(" Plant devices -> {}", res);

let res = client.mix_system_status(<mix-id>,<user-id>).await.unwrap();
println!(" MIX -> {}", res);
```

## Methods and Structures

### Methods

 * `login`
 * `device_list_by_plant`
 * `mix_system_status`

###  Structures

 * `MixStatus`

## Note
the project is highly inspired to [PyPi_GrowattServer](https://github.com/indykoning/PyPi_GrowattServer)
