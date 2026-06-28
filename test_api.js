const axios = require('axios');
const path = require('path');

async function test() {
    const instance = axios.create({
        baseURL: 'http://localhost:3000',
        withCredentials: true
    });

    try {
        // Login
        const loginRes = await instance.post('/api/auth/login', { username: 'admin', password: 'admin' });
        console.log('Login Success:', loginRes.status);
        const cookie = loginRes.headers['set-cookie'];
        instance.defaults.headers.Cookie = cookie;

        // Check story graph
        const graphRes = await instance.get('/api/admin/story-graph');
        console.log('Story Graph:', JSON.stringify(graphRes.data, null, 2));

        // Check uploads
        const uploadsRes = await instance.get('/api/scenes/uploads');
        console.log('Uploads:', JSON.stringify(uploadsRes.data, null, 2));

        // Check parts
        const partsRes = await instance.get('/api/parts');
        console.log('Parts:', JSON.stringify(partsRes.data, null, 2));

    } catch (err) {
        console.error('Error:', err.response ? err.response.data : err.message);
    }
}
test();
