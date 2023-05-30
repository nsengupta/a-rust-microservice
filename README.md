#### A simple implementation of a microservice

This is a simple microservice implementation, consisting of two services. The project itself is based on the course-exercise of Rust bootcamp, offered by Letsgorusty!

The application itself is straightforward. An authentication service offers its APIs to _SignUp_, _SignIn_ and 
_SignOut_ for users. The data is held in heap-resident Hashmaps (no DB yet). Additionally, a health-check service is run in the background. It keeps pinging the authentication service and checks if everything is OK. All the results of health checks, are displayed on the terminal (hence, it is run from a separate terminal).

The project uses Protobuf/gRPC for exchanging data. Additionally, docker images are built and run and, github actions are used for CI/

My intention is to expand the functionality, and include - among other things -
*   Integration with a datbase
*   Integration with REDIS