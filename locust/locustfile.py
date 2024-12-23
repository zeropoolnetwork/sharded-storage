from locust import HttpUser, task, between
import random

CLUSTERS = [
    "4a09785674d14344d92b1212b6e810369535ea1c",
    "dcfc37347dd5794515d7bb08ffcbca654f47d744",
    "dbf5013b65b95c339ecd6563acd4b8016cd0d80f",
    "486818732c691850ddcd5b241ca23319454fe575",
    "4bb7275205086e01c4bdef60113abd1c6c07b666",
    "d16bf0750fcda12088c406510f2d2f6c50d4097c",
    "f4468b46760db96b07658c71338db961fb6de72f",
    "6bfb0a0bfd71b41f71bd956b5e6af76c8ad5cd2b",
    "4a7fb852bb3f120e676f906c7e208e43f6dc1003",
    "ef4d97771d424720fb370d0e82f4537efb72c47a",
    "b8186e0e1806966514ea8d45b3eb3e7681bdf974",
    "c544b2178af4a4428cd1e12ca26d6428e3d24276",
]

class WebsiteUser(HttpUser):
    # Not useful for calculating maximum throughput
    # wait_time = between(1, 2)
    
    @task
    def get_clusters(self):
        random_cluster = random.choice(CLUSTERS)
        self.client.get(f"/clusters/{random_cluster}")