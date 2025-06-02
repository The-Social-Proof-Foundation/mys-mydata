// Copyright (c), Mysten Labs, Inc.
// Copyright (c), The Social Proof Foundation, LLC.
// SPDX-License-Identifier: Apache-2.0
import { getFullnodeUrl } from '@socialproof/mys/client';
import { TESTNET_PACKAGE_ID } from './constants';
import { createNetworkConfig } from '@socialproof/dapp-kit';

const { networkConfig, useNetworkVariable, useNetworkVariables } = createNetworkConfig({
  testnet: {
    url: getFullnodeUrl('testnet'),
    variables: {
      packageId: TESTNET_PACKAGE_ID,
      gqlClient: 'https://mys-testnet.mysocial.network/graphql',
    },
  },
});

export { useNetworkVariable, useNetworkVariables, networkConfig };
