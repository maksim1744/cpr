namespace main_sol {
stringstream cin, cout, cerr;
//->main
}

namespace check_sol {
stringstream cin, cout, cerr;
//->check
}

namespace gen_sol {
stringstream cin, cout, cerr;
//->gen
}

bool is_double(const string& s) {
    char* end = nullptr;
    double val = strtod(s.c_str(), &end);
    return end != s.c_str() && *end == '\0' && val != HUGE_VAL;
}

//->settings

int main() {
    ios_base::sync_with_stdio(false); cin.tie(NULL); cout.tie(NULL);

    for (int test = 1;; ++test) {
        int seed = start_seed + test - 1;
        stringstream sscerr;
        sscerr << "Case #" << test << " [seed=" << seed << "]: ";
        cerr << '\r' << sscerr.str() << string(3, ' ');
        cerr << '\r' << sscerr.str();
        gen_sol::cin = stringstream{};
        gen_sol::cout = stringstream{};
        gen_sol::cerr = stringstream{};
        main_sol::cin = stringstream{};
        main_sol::cout = stringstream{};
        main_sol::cerr = stringstream{};
        check_sol::cin = stringstream{};
        check_sol::cout = stringstream{};
        check_sol::cerr = stringstream{};

        char *argv[2];
        string s = to_string(seed);
        argv[1] = (char*)&s[0];
        gen_sol::main(2, argv);
        cout << "G"; cout.flush();
        main_sol::cin << gen_sol::cout.str();
        main_sol::main();
        cout << "M"; cout.flush();
        check_sol::cin << gen_sol::cout.str() << main_sol::cout.str();
        int result = check_sol::main();
        cout << "C"; cout.flush();

        if (result != 0) {
            cout << " failed";
            cout << endl;
            if (!quiet) {
                cout << "==========  in ==========" << endl;
                cout << gen_sol::cout.str();
                cout << "========== out ==========" << endl;
                cout << main_sol::cout.str();
                cout << "========== err ==========" << endl;
                cout << check_sol::cout.str();
            }

            {
                ofstream f("in");
                f << gen_sol::cout.str();
            }
            {
                ofstream f("out");
                f << main_sol::cout.str();
            }
            {
                ofstream f("err");
                f << check_sol::cout.str();
            }
            break;
        }
    }

    return 0;
}
