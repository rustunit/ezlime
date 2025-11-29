# x402 Signature Debugging Guide

## Current Error

```
invalid_exact_evm_payload_signature
```

This means the signature is valid format, but doesn't verify against the authorization. The issue is in how the frontend is signing the EIP-712 message.

## Correct ERC-3009 Signing (Base Sepolia)

### 1. EIP-712 Domain

```typescript
const domain = {
  name: "USD Coin",           // Must match USDC contract
  version: "2",               // USDC version
  chainId: 84532,             // Base Sepolia chain ID
  verifyingContract: "0x036CbD53842c5426634e7929541eC2318f3dCF7e" // USDC on Base Sepolia
};
```

### 2. EIP-712 Types

```typescript
const types = {
  TransferWithAuthorization: [
    { name: "from", type: "address" },
    { name: "to", type: "address" },
    { name: "value", type: "uint256" },
    { name: "validAfter", type: "uint256" },
    { name: "validBefore", type: "uint256" },
    { name: "nonce", type: "bytes32" }
  ]
};
```

### 3. Message (Authorization)

```typescript
const authorization = {
  from: "0x690852e31515f0c9ff8308009f5d63ea2d346a09",  // Your address
  to: "0xe3280d444d7afe2a83387972ef5514d80dd61d49",    // Merchant wallet
  value: "5000",                                        // Amount in base units (string)
  validAfter: "0",                                      // Unix timestamp (string)
  validBefore: "999999999999",                          // Unix timestamp (string)
  nonce: ethers.hexlify(ethers.randomBytes(32))        // Random 32 bytes
};
```

### 4. Sign

```typescript
const signature = await signer.signTypedData(domain, types, authorization);
```

## Common Mistakes

### ❌ Wrong domain name
```typescript
name: "USDC"  // Wrong - should be "USD Coin"
```

### ❌ Wrong version
```typescript
version: "1"  // Wrong - USDC on Base Sepolia uses "2"
```

### ❌ Wrong chain ID
```typescript
chainId: 8453   // Wrong - that's Base mainnet, use 84532 for Sepolia
```

### ❌ Numbers instead of strings
```typescript
value: 5000,           // Wrong - must be string
validAfter: 0,         // Wrong - must be string
validBefore: 999999,   // Wrong - must be string
```

### ❌ Wrong nonce format
```typescript
nonce: "12345"  // Wrong - must be 32 bytes hex (0x + 64 chars)
```

## How to Get Correct Domain Info

Query the USDC contract:

```typescript
const usdcContract = new ethers.Contract(
  "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
  [
    "function name() view returns (string)",
    "function version() view returns (string)",
    "function DOMAIN_SEPARATOR() view returns (bytes32)"
  ],
  provider
);

const name = await usdcContract.name();
const version = await usdcContract.version();
const domainSeparator = await usdcContract.DOMAIN_SEPARATOR();

console.log({ name, version, domainSeparator });
```

**Expected output:**
```
{
  name: "USD Coin",
  version: "2",
  domainSeparator: "0x..." 
}
```

## Complete Working Example

```typescript
import { ethers } from 'ethers';

async function createValidAuthorization(
  signer: ethers.Signer,
  merchantAddress: string,
  amount: string
) {
  // 1. Get USDC contract info
  const usdcAddress = "0x036CbD53842c5426634e7929541eC2318f3dCF7e";
  const usdcContract = new ethers.Contract(
    usdcAddress,
    [
      "function name() view returns (string)",
      "function version() view returns (string)"
    ],
    signer
  );

  // 2. Build EIP-712 domain
  const domain = {
    name: await usdcContract.name(),     // "USD Coin"
    version: await usdcContract.version(), // "2"
    chainId: 84532,                       // Base Sepolia
    verifyingContract: usdcAddress
  };

  console.log("Domain:", domain);

  // 3. Define types
  const types = {
    TransferWithAuthorization: [
      { name: "from", type: "address" },
      { name: "to", type: "address" },
      { name: "value", type: "uint256" },
      { name: "validAfter", type: "uint256" },
      { name: "validBefore", type: "uint256" },
      { name: "nonce", type: "bytes32" }
    ]
  };

  // 4. Build authorization
  const from = await signer.getAddress();
  const now = Math.floor(Date.now() / 1000);
  
  const authorization = {
    from: from,
    to: merchantAddress,
    value: amount,  // Already in base units
    validAfter: "0",
    validBefore: (now + 3600).toString(),  // Valid for 1 hour
    nonce: ethers.hexlify(ethers.randomBytes(32))
  };

  console.log("Authorization:", authorization);

  // 5. Sign
  const signature = await signer.signTypedData(domain, types, authorization);
  
  console.log("Signature:", signature);
  console.log("Signature length:", signature.length); // Should be 132

  return {
    signature,
    authorization
  };
}
```

## Verification Test

Your frontend should log:
```
Domain: {
  name: 'USD Coin',
  version: '2',
  chainId: 84532,
  verifyingContract: '0x036CbD53842c5426634e7929541eC2318f3dCF7e'
}
Authorization: {
  from: '0x690852e31515f0c9ff8308009f5d63ea2d346a09',
  to: '0xe3280d444d7afe2a83387972ef5514d80dd61d49',
  value: '5000',
  validAfter: '0',
  validBefore: '1764451695',
  nonce: '0x8d25ade5156ee69003ee6e889f2d58bbe77d8415858af7de5bab3799c3593480'
}
Signature: 0x35b2b9b3f3748e537da48d13fb4e7b281a0978480839309f96d5d1009621b78343ee35557f7f76824ef7914daddfc99f092635e57dd55ea06ead9543fc16043a1b
Signature length: 132
```

## Testing Locally

You can test if the signature is correct by calling the USDC contract's view function:

```typescript
// This should NOT revert if signature is valid
const isValid = await usdcContract.callStatic.transferWithAuthorization(
  authorization.from,
  authorization.to,
  authorization.value,
  authorization.validAfter,
  authorization.validBefore,
  authorization.nonce,
  signature.v,
  signature.r,
  signature.s
);
```

## Next Steps

1. **Query the USDC contract** to verify domain name and version
2. **Log all signing parameters** in your frontend console
3. **Ensure all values are strings** (not numbers)
4. **Check chain ID is 84532** (Base Sepolia)
5. **Test the signature locally** before sending to backend

The issue is almost certainly in the EIP-712 domain or message structure in your frontend.
