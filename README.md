# Virtual Story

Virtual Story is an interactive, "choose your own adventure" video application. It allows users to navigate through a branching narrative by making choices at the end of video segments, creating a personalized story experience. This project is inspired by platforms like LifeSelector.

## Technology Stack

- **Frontend:** Vue.js
- **Backend:** Node.js with Express.js
- **Database:** MariaDB

## How It Works

A story is constructed from multiple short video segments. At the end of each segment, the viewer is presented with choices that lead to different video segments, allowing the story to branch. The narrative is managed by a decision tree stored in the database.

## Installation

Follow these steps to set up the project for development.

### 1. Prerequisites

Ensure you have the following software installed on your system:

-   **Node.js:** (v18 or later recommended)
-   **npm:** (comes with Node.js)
-   **MariaDB:** A running instance of the MariaDB server.
-   **ffmpeg:** This is required for video processing tasks in the administration panel (e.g., generating thumbnails). You can install it using your system's package manager (e.g., `apt`, `brew`, `choco`).

### 2. Clone the Repository

```bash
git clone <repository-url>
cd virtual-story
```

### 3. Database Setup

You need to create a dedicated database and user for the application.

1.  Log in to your MariaDB server as a user with sufficient privileges (e.g., `root`):
    ```bash
    mysql -u root -p
    ```

2.  Run the following SQL commands to create the database and a new user. Replace `'your_db_user'` and `'your_db_password'` with secure credentials.

    ```sql
    CREATE DATABASE virtualstory;
    CREATE USER 'your_db_user'@'localhost' IDENTIFIED BY 'your_db_password';
    GRANT ALL PRIVILEGES ON virtualstory.* TO 'your_db_user'@'localhost';
    FLUSH PRIVILEGES;
    EXIT;
    ```

### 4. Backend Setup

1.  Navigate to the backend directory:
    ```bash
    cd backend
    ```

2.  Copy the example environment file:
    ```bash
    cp .env.example .env
    ```

3.  Edit the new `.env` file and replace the placeholder values with the database credentials you just created.

4.  Install the dependencies:
    ```bash
    npm install
    ```

### 5. Frontend Setup

1.  Navigate to the frontend directory from the project root:
    ```bash
    cd frontend
    ```

2.  Install the dependencies:
    ```bash
    npm install
    ```

## How to Run

1.  **Initialize the Database Schema:**
    Before starting the server for the first time, you must create the necessary tables. From the project's **root directory**, run:
    ```bash
    node backend/init-db.js
    ```

2.  **Start the Backend Server:**
    From the **root directory**, run:
    ```bash
    node backend/server.js
    ```
    The server will start on `http://localhost:3000`.

3.  **Start the Frontend Development Server:**
    In a new terminal, navigate to the **frontend directory** and run:
    ```bash
    npm run dev
    ```
    The application will be accessible at `http://localhost:5173` (or the next available port).

## Administration Interface

The application includes a planned administration interface to manage the story's decision tree. This interface will provide a visual representation of the video connections.
