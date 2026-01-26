#!/bin/bash

echo "Testing cookie extraction fix for subdomain matching..."
echo ""

# Test 1: areena.yle.fi (subdomain - should find parent domain cookies on .yle.fi)
echo "Test 1: Fetching areena.yle.fi (subdomain)"
COOKIES_SUBDOMAIN=$(RUST_LOG=info ./target/release/microfetch fetch https://areena.yle.fi --cookies brave 2>&1 | grep "Loaded.*cookies" || echo "0 cookies")
echo "Result: $COOKIES_SUBDOMAIN"
echo ""

# Test 2: yle.fi (parent domain - should also find cookies on .yle.fi)  
echo "Test 2: Fetching yle.fi (parent domain)"
COOKIES_PARENT=$(RUST_LOG=info ./target/release/microfetch fetch https://yle.fi --cookies brave 2>&1 | grep "Loaded.*cookies" || echo "0 cookies")
echo "Result: $COOKIES_PARENT"
echo ""

# Verify both found cookies
if [[ "$COOKIES_SUBDOMAIN" == *"0 cookies"* ]]; then
    echo "❌ FAILED: Subdomain test found 0 cookies (should find parent domain cookies)"
    exit 1
elif [[ "$COOKIES_PARENT" == *"0 cookies"* ]]; then
    echo "❌ FAILED: Parent domain test found 0 cookies"
    exit 1
else
    echo "✅ SUCCESS: Both tests found cookies correctly!"
    echo "   - Subdomain (areena.yle.fi) correctly matches parent domain (.yle.fi) cookies"
    echo "   - Parent domain (yle.fi) correctly matches its own (.yle.fi) cookies"
fi
