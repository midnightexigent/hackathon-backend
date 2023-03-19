# backend 

http server that returns a list of whitelisted vendors and executes transactions 

## run

`RUST_LOG=trace cargo run -- <PORT>`

## requests

```sh
# add vendor 
curl -H "Content-Type: application/json" --data '{"wallet_id": "1234324","name": "toto"}' localhost:3030/vendors

# retrieve all vendors
curl localhost:3030/vendors

# execute buy
curl -H "Content-Type: application/json" --data '{"lamports": 12312, "vendor": "1234324", "buyer_pair": "123234243"}' localhost:3030/buy
```
