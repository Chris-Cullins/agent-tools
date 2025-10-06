// Test JavaScript file for ast-find

import axios from 'axios';
import { get } from 'requests';

async function fetchData() {
    const response = await axios.get('https://api.example.com/data');
    return response.data;
}

async function postData(payload) {
    const result = await axios.post('https://api.example.com/data', payload);
    return result;
}

function regularFunction() {
    console.log('Hello world');
}

class MyClass {
    constructor() {
        this.value = 42;
    }

    getValue() {
        return this.value;
    }
}

export { fetchData, postData };
