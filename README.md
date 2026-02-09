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
-   **pnpm:** (used for dependency management)
-   **MariaDB:** A running instance of the MariaDB server.
-   **ffmpeg:** Required for video processing tasks in the administration panel (e.g., generating thumbnails). You can install it using your system's package manager (e.g., `apt`, `brew`, `choco`).

### 2. Clone the Repository

```bash
git clone <repository-url>
cd virtual-story
```

### 3. Database Setup

Create a dedicated database and user for the application.

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
    - **Note:** You should also set a unique `SESSION_SECRET` for security.

4.  Install the dependencies:
    ```bash
    pnpm install
    ```

### 5. Frontend Setup

1.  Navigate to the frontend directory from the project root:
    ```bash
    cd frontend
    ```

2.  Install the dependencies:
    ```bash
    pnpm install
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
    pnpm dev
    ```
    The application will be accessible at `http://localhost:5173` (or the next available port).

## Usage Tutorial

The administration interface allows you to build and visualize your interactive story. It is protected by authentication to ensure only authorized users can modify the story.

### Security and Authentication

-   **Initial Credentials:** After initializing the database, the default user is `admin` with the password `admin`.
-   **Logging In:** Click the **"Admin"** link in the header. If you are not logged in, a modal will appear requesting your credentials.
-   **Changing Password:** Once logged in, go to the **"Admin"** section and click the **"Utilisateurs & Profil"** button. Here you can change your password by providing your old password and a new one.
-   **Managing Users:** The same section allows you to create new administrative users or delete existing ones (except your own account).
-   **Session Security:** Authentication is handled via secure cookies. Brute-force protection is implemented (10 attempts allowed every 15 minutes).

### Main View: Story Graph

The main admin page displays the "Story Graph", a hierarchical view showing connections between all your scenes.

-   **Root Scenes:** Scenes without a parent are displayed at the top level.
-   **Child Scenes:** Scenes linked by a choice are nested under their parent scene.
-   **Chapter Badges:** Scenes associated with a chapter display a badge with the chapter name.

Each scene in the graph has buttons to **"Edit"** (modify details and links) or **"View"** (preview it in the player).

![Story Graph](docs/images/admin_story_graph.png)

### Managing Scenes

1.  **Add a Root Scene:** Use the **"Add Root Scene"** button to create a new scene that will be a starting point for a narrative branch.
2.  **Sequential Creation:** When creating a scene, you stay on the form page after saving. This allows you to quickly enter multiple scenes in a row without returning to the list.
3.  **Edit a Scene:** On the edit page, you can:
    -   Modify the scene title and associate it with a **Chapter**.
    -   Upload a new video and its thumbnail.
    -   **Add a choice (child):** Link this scene to another by creating a choice.
    -   **Link a parent:** Create an incoming link from another scene, making this one its child.

### Chapter Management (Parts)

The "Parts" system organizes your narrative into distinct chapters.

-   **Creation:** Give your chapter a title and select its starting scene.
-   **Editing:** You can rename a chapter or change its starting scene at any time.
-   **Ambient Loop Video:** You can upload one looping video per chapter. This video is displayed in the background of the player (left panel) to enhance immersion.
-   **Navigation:** Chapters appear in the application header for quick access.

### Database Synchronization

If you encounter "Unknown column" errors or after updating the application, use the **"Synchronize Database"** button in the Administration tab. This tool updates your SQL schema automatically.

### Player Experience

The player is optimized for full immersion:
-   **Full-Page Mode:** Videos automatically display in "full-page" mode (CSS overlay) to hide the interface during playback.
-   **Smart Autoplay:** The application attempts to start the video with sound, falling back to muted mode if the browser blocks automatic playback.
-   **Narrative Navigation:** The "Previous Scenes" button allows you to go back in the story flow rather than just the browser history.

### Customizing the Background

In the **"Player Background"** section, you can upload a global background image displayed behind the player interface.

## Author

- **Martial Limousin** - martial.limousin@gmail.com
