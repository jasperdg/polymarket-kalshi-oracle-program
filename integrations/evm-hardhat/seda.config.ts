export interface SedaConfig {
  coreAddress: string;
}

export const networkConfigs: { [network: string]: SedaConfig } = {
  // Proxy Core Addresses (SEDA mainnet)
  base: {
    coreAddress: '0xDF1fb5ACe711B16D90FC45776fF1bF02CEBc245D',
  },
  // Proxy Core Addresses (SEDA testnet)
  baseSepolia: {
    coreAddress: '0xffDB1d9bBE4D56780143428450c4C2058061E6F3',
  },
  superseedSepolia: {
    coreAddress: '0xE08989FB730E072689b4885c2a62AE5f1fc787F2',
  },
  chiado: {
    coreAddress: '0xbe2ace709959C121759d553cACf7e6532C25a3aA',
  },
  hyperliquidPurrsec: {
    coreAddress: '0x23c01fe3C1b7409A98bBd39a7c9e5C2263C64b59',
  },
};
