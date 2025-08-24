const express = require('express');
const https = require('https');
const fs = require('fs');
const path = require('path');

const app = express();
const PORT = 4433; // Or any other desired port

// Path to your SSL certificate and key
const options = {
    key: fs.readFileSync(path.join(__dirname, 'localhost-key.pem')),
    cert: fs.readFileSync(path.join(__dirname, 'localhost.pem'))
};

// Serve static files from a specific directory (e.g., 'public')
// Create a 'public' directory in your project and place your files there.
app.use(express.static(path.join(__dirname, '.')));

// Create the HTTPS server
https.createServer(options, app).listen(PORT, () => {
    console.log(`HTTPS server running on https://localhost:${PORT}`);
});
