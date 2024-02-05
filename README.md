## Uniswap v2 balance checker


This application gathers and compares liquidity pool data from the Polygon network's Uniswap v2 pools. It identifies discrepancies between pool reserves data retrieved via getReserves and balanceOf, presenting the findings in a detailed JSON format for each pool, highlighting any inconsistencies.

#### Prerequisites
RPC NODE can return archived data
 
### Usage

#### Configuration
`mv .env.example .env`

Run with load actual pools from cache

`cargo run`

Run with fetching actual pools from the network

`cargo run --release`

Output will be stored in [output](output) folder.