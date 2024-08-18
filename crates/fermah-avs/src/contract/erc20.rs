use ethers::contract::abigen;

abigen!(IERC20, "../../contracts/out/IERC20.sol/IERC20.json");

abigen!(ERC20, "../../contracts/out/ERC20.sol/ERC20.json");

#[cfg(any(feature = "mock_strategy", feature = "mock_vault_token"))]
abigen!(
    ERC20Mock,
    "../../contracts/out/ERC20Mock.sol/ERC20Mock.json"
);
