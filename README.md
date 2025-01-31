# WebSocket Authentication Service

A **secure** and **scalable** WebSocket authentication service built with **Rust**, **Axum**, and **PostgreSQL**. This project provides robust middleware for authenticating WebSocket connections using JWT tokens and efficiently managing user sessions.

## ✨ Features

- **JWT-based Authentication**: Secure WebSocket connections using JSON Web Tokens (JWT).
- **Session Management**: Track active user sessions with expiration.
- **Password Strength Validation**: Ensure strong passwords during user registration.
- **WebSocket Middleware**: Protect WebSocket routes with an authentication middleware.
- **PostgreSQL Integration**: Store user data and sessions in a PostgreSQL database.

---

## Getting Started

### Prerequisites

Before running the application, ensure you have the following installed:

- **Rust**: Install Rust 
- **PostgreSQL**: Install PostgreSQL and create a database for the application.
- **Environment Variables**: Set up a `.env` file with the required configurations.

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-username/websocket-auth-service.git
   cd websocket-auth-service
   ```

2. **Install dependencies:**
   ```bash
   cargo build
   ```

3. **Run using Docker:**
   ```bash
   docker-compose build
   ```

---

## Configuration

1. **Create a `.env` file** in the root directory with the following variables:
   ```env
   DATABASE_URL=postgres://username:password@localhost/database_name
   JWT_SECRET=your_jwt_secret_key
   ```

2. **Run the application:**
   ```bash
   docker compose up -d
   ```

The server will start on [http://localhost:3000](http://localhost:3000).

---

## API Endpoints

### Authentication

#### Register a new user
```http
POST /register
```
##### Request Body:
```json
{
  "username": "user123",
  "email": "user@example.com",
  "password": "StrongPassword123!"
}
```

#### Log in and receive a JWT token
```http
POST /login
```
##### Request Body:
```json
{
  "username": "user123",
  "password": "StrongPassword123!"
}
```

### WebSocket

#### Connect to a WebSocket endpoint
```http
GET /ws
```
Requires a valid JWT token in the `Authorization` header or as a query parameter.

#### WebSocket Authentication
To authenticate a WebSocket connection, include the JWT token in one of the following ways:

- **Authorization Header:**
  ```http
  Authorization: Bearer <JWT_TOKEN>
  ```

- **Query Parameter:**
  ```http
  /ws?token=<JWT_TOKEN>&chat_id=<CHAT_ID>
  ```

The middleware will verify the token and allow the connection if valid.

## Contributing

Contributions are welcome! Follow these steps:

1. **Fork the repository.**
2. **Create a new branch:**
   ```bash
   git checkout -b feature/your-feature
   ```
3. **Commit your changes:**
   ```bash
   git commit -m "Add some feature"
   ```
4. **Push to the branch:**
   ```bash
   git push origin feature/your-feature
   ```
5. **Open a pull request.**

Please ensure your code follows the project's coding standards and includes tests where applicable.

---

## License

This project is licensed under the **MIT License**. See the `LICENSE` file for details.

---

## Acknowledgments
Special thanks to all contributors and the open-source community!



