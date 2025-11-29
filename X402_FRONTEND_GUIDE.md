# x402 Frontend Integration Guide

## Overview

The backend now implements the **official x402 specification** using ERC-3009 transfer authorizations. Your frontend needs to create signed transfer authorizations, NOT execute on-chain transactions.

## Key Change

**OLD (Wrong):**
- Frontend executes an ERC-20 `transfer()` transaction
- Sends transaction hash to backend
- ❌ This doesn't work with x402

**NEW (Correct):**
- Frontend creates an **ERC-3009 transfer authorization** (off-chain signature)
- Sends the authorization to backend
- Backend's facilitator executes the transaction
- ✅ This is the real x402 protocol

## Step-by-Step Frontend Implementation

### 1. Get Payment Requirements

**Request:**
```bash
POST /x402/shorten
Content-Type: application/json

{
  "url": "https://example.com/my-long-url"
}
```

**Response (402):**
```json
{
  "x402Version": 1,
  "accepts": [{
    "scheme": "exact",
    "network": "base-sepolia",
    "maxAmountRequired": "5000",
    "asset": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
    "payTo": "0xMerchantWallet",
    "resource": "/x402/shorten",
    "description": "Link shortening service",
    "maxTimeoutSeconds": 60
  }]
}
```

### 2. Create ERC-3009 Transfer Authorization

**Using ethers.js v6:**

```typescript
import { ethers } from 'ethers';

// ERC-3009 USDC contract ABI (only the methods we need)
const USDC_ABI = [
  "function transferWithAuthorization(address from, address to, uint256 value, uint256 validAfter, uint256 validBefore, bytes32 nonce, uint8 v, bytes32 r, bytes32 s) external",
  "function TRANSFER_WITH_AUTHORIZATION_TYPEHASH() view returns (bytes32)",
  "function DOMAIN_SEPARATOR() view returns (bytes32)",
  "function name() view returns (string)",
  "function version() view returns (string)"
];

async function createTransferAuthorization(
  signer: ethers.Signer,
  paymentReq: PaymentRequirement
) {
  const usdcContract = new ethers.Contract(
    paymentReq.asset,
    USDC_ABI,
    signer
  );

  const from = await signer.getAddress();
  const to = paymentReq.payTo;
  const value = paymentReq.maxAmountRequired;
  const validAfter = Math.floor(Date.now() / 1000);
  const validBefore = validAfter + (paymentReq.maxTimeoutSeconds || 60);
  
  // Generate random nonce
  const nonce = ethers.hexlify(ethers.randomBytes(32));

  // Get EIP-712 domain
  const domain = {
    name: await usdcContract.name(),
    version: await usdcContract.version(),
    chainId: 84532, // Base Sepolia
    verifyingContract: paymentReq.asset
  };

  // EIP-712 types for TransferWithAuthorization
  const types = {
    TransferWithAuthorization: [
      { name: 'from', type: 'address' },
      { name: 'to', type: 'address' },
      { name: 'value', type: 'uint256' },
      { name: 'validAfter', type: 'uint256' },
      { name: 'validBefore', type: 'uint256' },
      { name: 'nonce', type: 'bytes32' }
    ]
  };

  // Create the authorization message
  const authorization = {
    from,
    to,
    value,
    validAfter: validAfter.toString(),
    validBefore: validBefore.toString(),
    nonce
  };

  // Sign the authorization (EIP-712)
  const signature = await signer.signTypedData(domain, types, authorization);

  return {
    signature,
    authorization
  };
}
```

### 3. Build X-PAYMENT Header

```typescript
function buildXPaymentHeader(
  authorization: any,
  signature: string,
  network: string
) {
  const paymentPayload = {
    x402Version: 1,
    scheme: "exact",
    network: network,
    payload: {
      signature: signature,
      authorization: {
        from: authorization.from,
        to: authorization.to,
        value: authorization.value,
        validAfter: authorization.validAfter,
        validBefore: authorization.validBefore,
        nonce: authorization.nonce
      }
    }
  };

  // Base64 encode the JSON
  const json = JSON.stringify(paymentPayload);
  return btoa(json);
}
```

### 4. Retry Request with Payment

```typescript
async function createShortLink(url: string, signer: ethers.Signer) {
  // Step 1: Get payment requirements
  const response = await fetch('/x402/shorten', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ url })
  });

  if (response.status === 402) {
    const paymentRequired = await response.json();
    const paymentReq = paymentRequired.accepts[0];

    // Step 2: Create authorization
    const { authorization, signature } = await createTransferAuthorization(
      signer,
      paymentReq
    );

    // Step 3: Build X-PAYMENT header
    const xPayment = buildXPaymentHeader(
      authorization,
      signature,
      paymentReq.network
    );

    // Step 4: Retry with payment
    const paidResponse = await fetch('/x402/shorten', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-PAYMENT': xPayment
      },
      body: JSON.stringify({ url })
    });

    if (paidResponse.ok) {
      const result = await paidResponse.json();
      console.log('Short link created:', result.short_url);
      return result;
    } else {
      throw new Error('Payment failed');
    }
  }
}
```

## Important Notes

### Amount Format

The `maxAmountRequired` is in **token base units** (not decimals):
- USDC has 6 decimals
- `"5000"` = 0.005 USDC = $0.005
- To convert: `amount * 10^6`

Example:
```typescript
const usdcAmount = 0.005; // $0.005
const baseUnits = Math.floor(usdcAmount * 1e6).toString(); // "5000"
```

### Nonce Generation

The nonce must be:
- **Random** (use `crypto.randomBytes(32)` or `ethers.randomBytes(32)`)
- **Unique** per authorization
- **32 bytes** (bytes32)

### Signature Format

The signature from `signTypedData()` is:
- 65 bytes hex string
- Format: `0x${r}${s}${v}`
- Send as-is to backend

### Network Names

Use exact network names:
- `"base-sepolia"` for testnet
- `"base"` for mainnet

## Testing

You can test your frontend with this curl command (with a real authorization):

```bash
# Get payment requirements
curl -X POST http://localhost:8080/x402/shorten \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'

# Pay and retry (replace with your actual base64 payment payload)
curl -X POST http://localhost:8080/x402/shorten \
  -H "Content-Type: application/json" \
  -H "X-PAYMENT: eyJ4NDAyVmVyc2lvbiI6MSwic2NoZW1lIjoiZXhhY3QiLC4uLn0=" \
  -d '{"url": "https://example.com"}'
```

## Common Errors

### "Invalid JSON in payment payload"
- Check base64 encoding is correct
- Verify JSON structure matches spec exactly
- Use camelCase for all fields

### "Payment verification failed"
- Signature might be invalid
- Check EIP-712 domain is correct
- Verify authorization timestamps are valid
- Ensure nonce hasn't been used before

### "Facilitator returned 500"
- Check network name matches exactly
- Verify USDC contract address is correct
- Ensure amount is in base units, not decimals

## Libraries

Recommended libraries for x402 integration:
- **ethers.js v6** - For wallet connection and signing
- **viem** - Alternative to ethers with better TypeScript support
- **wagmi** - React hooks for Web3, includes EIP-712 signing

## Example: Complete React Component

```typescript
import { useWallet } from '@/hooks/useWallet';
import { useState } from 'react';

export function ShortenLinkButton({ url }: { url: string }) {
  const { signer } = useWallet();
  const [loading, setLoading] = useState(false);
  const [shortUrl, setShortUrl] = useState<string | null>(null);

  async function handleShorten() {
    setLoading(true);
    try {
      const result = await createShortLink(url, signer);
      setShortUrl(result.short_url);
    } catch (error) {
      console.error('Failed to shorten link:', error);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <button onClick={handleShorten} disabled={loading}>
        {loading ? 'Processing payment...' : 'Shorten with x402'}
      </button>
      {shortUrl && <p>Short URL: {shortUrl}</p>}
    </div>
  );
}
```

## Summary

✅ Create ERC-3009 transfer authorization (off-chain signature)  
✅ Build proper x402 payment payload with authorization  
✅ Base64 encode and send in X-PAYMENT header  
❌ Do NOT execute on-chain transfer yourself  
❌ Do NOT send transaction hash  

The facilitator handles the on-chain execution after verification.
