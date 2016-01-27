#include <boost/asio/connect.hpp>
#include <boost/asio/io_service.hpp>
#include <boost/asio/ip/tcp.hpp>
#include <boost/asio/ip/udp.hpp>
#include <boost/asio/write.hpp>
#include <boost/lexical_cast.hpp>

int main(int argc, char** argv) {
    boost::asio::io_service loop;

    typedef boost::asio::ip::tcp protocol_type;
    protocol_type::socket socket(loop);
    protocol_type::resolver resolver(loop);
    boost::asio::connect(socket, resolver.resolve({argv[1], argv[2]}));

    std::string message = R"({"id":42,"source":"core","nested":{"key":"value"},"message":"le message - )";
    std::string data;
    data.reserve(512);
    uint count = boost::lexical_cast<uint>(argv[3]);
    for (uint i = 0; i < count; ++i) {
        data.assign(message);
        data.append(boost::lexical_cast<std::string>(i));
        data.append("\"}");
        // socket.send(boost::asio::buffer(data));
        boost::asio::write(socket, boost::asio::buffer(data));
    }

    return 0;
}
