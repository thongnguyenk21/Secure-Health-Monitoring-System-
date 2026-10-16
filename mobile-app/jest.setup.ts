// Screens read the backend URL at import time; tests always talk to a mocked fetch.
process.env.EXPO_PUBLIC_API_URL = "http://api.test";
